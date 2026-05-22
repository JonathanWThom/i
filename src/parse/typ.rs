use super::cursor::Cursor;
use crate::ast::{EffectRow, Type, TypeKind};
use crate::error::{Error, ErrorKind};
use crate::span::Spanned;
use crate::token::TokenKind;

pub(super) fn parse_type(cur: &mut Cursor) -> Result<Type, Error> {
    let start = cur.peek().span;
    let first = parse_type_atom_with_args(cur)?;
    let mut atoms = vec![first];
    while cur.eat(&TokenKind::Comma) {
        atoms.push(parse_type_atom_with_args(cur)?);
    }

    let pre_eff = if cur.check(&TokenKind::Bang) && is_pre_arrow_effect(cur) {
        cur.bump();
        Some(parse_effect_row(cur)?)
    } else {
        None
    };

    if cur.eat(&TokenKind::Arrow) {
        let result = parse_type(cur)?;
        let result_end = result.span;
        let effect = if pre_eff.is_none() && cur.check(&TokenKind::Bang) {
            cur.bump();
            Some(parse_effect_row(cur)?)
        } else {
            pre_eff
        };
        return Ok(Spanned {
            span: start.merge(result_end),
            node: TypeKind::Function {
                params: atoms,
                effect,
                result: Box::new(result),
            },
        });
    }

    if pre_eff.is_some() {
        return Err(Error {
            span: cur.peek().span,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", cur.peek_kind()),
                expected: "`->` after effect row",
            },
        });
    }

    if atoms.len() == 1 {
        Ok(atoms.into_iter().next().unwrap())
    } else {
        let last = atoms.last().unwrap().span;
        Ok(Spanned {
            span: start.merge(last),
            node: TypeKind::Tuple(atoms),
        })
    }
}

fn parse_type_atom_with_args(cur: &mut Cursor) -> Result<Type, Error> {
    let start = cur.peek().span;
    match cur.peek_kind().clone() {
        TokenKind::LowerIdent(n) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: TypeKind::Var(n),
            })
        }
        TokenKind::UpperIdent(n) => {
            cur.bump();
            let mut args = Vec::new();
            let mut end_span = start;
            while starts_type_atom(cur.peek_kind()) {
                let a = parse_type_atom(cur)?;
                end_span = a.span;
                args.push(a);
            }
            Ok(Spanned {
                span: start.merge(end_span),
                node: TypeKind::Named { name: n, args },
            })
        }
        TokenKind::LParen => parse_paren(cur),
        other => Err(Error {
            span: start,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", other),
                expected: "a type",
            },
        }),
    }
}

fn parse_type_atom(cur: &mut Cursor) -> Result<Type, Error> {
    let start = cur.peek().span;
    match cur.peek_kind().clone() {
        TokenKind::LowerIdent(n) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: TypeKind::Var(n),
            })
        }
        TokenKind::UpperIdent(n) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: TypeKind::Named {
                    name: n,
                    args: vec![],
                },
            })
        }
        TokenKind::LParen => parse_paren(cur),
        other => Err(Error {
            span: start,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", other),
                expected: "a type",
            },
        }),
    }
}

fn parse_paren(cur: &mut Cursor) -> Result<Type, Error> {
    let start = cur.peek().span;
    cur.bump();
    let inner = parse_type(cur)?;
    let close = cur.expect(TokenKind::RParen, "`)`")?.span;
    Ok(Spanned {
        span: start.merge(close),
        node: inner.node,
    })
}

fn starts_type_atom(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::LowerIdent(_) | TokenKind::UpperIdent(_) | TokenKind::LParen
    )
}

fn is_pre_arrow_effect(cur: &Cursor) -> bool {
    let mut i = 1;
    match cur.peek_n(i) {
        Some(TokenKind::LParen) => {
            i += 1;
            if !matches!(cur.peek_n(i), Some(TokenKind::RParen)) {
                return false;
            }
            i += 1;
        }
        Some(TokenKind::UpperIdent(_)) => {
            i += 1;
            while matches!(cur.peek_n(i), Some(TokenKind::Comma)) {
                i += 1;
                if !matches!(cur.peek_n(i), Some(TokenKind::UpperIdent(_))) {
                    return false;
                }
                i += 1;
            }
        }
        _ => return false,
    }
    matches!(cur.peek_n(i), Some(TokenKind::Arrow))
}

fn parse_effect_row(cur: &mut Cursor) -> Result<EffectRow, Error> {
    if cur.eat(&TokenKind::LParen) {
        cur.expect(TokenKind::RParen, "`)`")?;
        return Ok(EffectRow::Empty);
    }
    let mut names = Vec::new();
    loop {
        let span = cur.peek().span;
        match cur.peek_kind().clone() {
            TokenKind::UpperIdent(n) => {
                cur.bump();
                names.push(n);
            }
            other => {
                return Err(Error {
                    span,
                    kind: ErrorKind::Unexpected {
                        found: format!("{:?}", other),
                        expected: "effect name or `(`",
                    },
                });
            }
        }
        if !cur.eat(&TokenKind::Comma) {
            break;
        }
    }
    Ok(EffectRow::Named(names))
}
