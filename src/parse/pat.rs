use super::cursor::Cursor;
use super::postfix::looks_like_kwargs;
use crate::ast::{FieldPat, LitPat, Pattern, PatternKind};
use crate::error::{Error, ErrorKind};
use crate::span::Spanned;
use crate::token::TokenKind;

pub(super) fn parse_pattern(cur: &mut Cursor) -> Result<Pattern, Error> {
    let atom = parse_pattern_atom(cur)?;
    if !starts_pattern(cur.peek_kind()) {
        return Ok(atom);
    }
    let atom_span = atom.span;
    match atom.node {
        PatternKind::Ctor { name, .. } => {
            let mut args = vec![parse_pattern_atom(cur)?];
            while cur.eat(&TokenKind::Comma) {
                args.push(parse_pattern_atom(cur)?);
            }
            let end_span = args.last().unwrap().span;
            Ok(Spanned {
                span: atom_span.merge(end_span),
                node: PatternKind::Ctor { name, args },
            })
        }
        other => Ok(Spanned {
            span: atom_span,
            node: other,
        }),
    }
}

fn parse_pattern_atom(cur: &mut Cursor) -> Result<Pattern, Error> {
    let start = cur.peek().span;
    match cur.peek_kind().clone() {
        TokenKind::Underscore => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: PatternKind::Wildcard,
            })
        }
        TokenKind::LowerIdent(n) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: PatternKind::Var(n),
            })
        }
        TokenKind::IntLit(n) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: PatternKind::Lit(LitPat::Int(n)),
            })
        }
        TokenKind::FloatLit(n) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: PatternKind::Lit(LitPat::Float(n)),
            })
        }
        TokenKind::StringLit(s) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: PatternKind::Lit(LitPat::Str(s)),
            })
        }
        TokenKind::UpperIdent(name) => {
            cur.bump();
            if cur.check(&TokenKind::LParen) && looks_like_kwargs(cur) {
                cur.bump();
                let fields = parse_field_patterns(cur)?;
                let close = cur.expect(TokenKind::RParen, "`)`")?.span;
                Ok(Spanned {
                    span: start.merge(close),
                    node: PatternKind::Record {
                        type_name: name,
                        fields,
                    },
                })
            } else {
                Ok(Spanned {
                    span: start,
                    node: PatternKind::Ctor { name, args: vec![] },
                })
            }
        }
        TokenKind::LParen => {
            cur.bump();
            let mut items = vec![parse_pattern(cur)?];
            while cur.eat(&TokenKind::Comma) {
                items.push(parse_pattern(cur)?);
            }
            let close = cur.expect(TokenKind::RParen, "`)`")?.span;
            Ok(Spanned {
                span: start.merge(close),
                node: PatternKind::Tuple(items),
            })
        }
        TokenKind::LBracket => {
            cur.bump();
            let mut items = Vec::new();
            if !cur.check(&TokenKind::RBracket) {
                items.push(parse_pattern(cur)?);
                while cur.eat(&TokenKind::Comma) {
                    items.push(parse_pattern(cur)?);
                }
            }
            let close = cur.expect(TokenKind::RBracket, "`]`")?.span;
            Ok(Spanned {
                span: start.merge(close),
                node: PatternKind::List(items),
            })
        }
        other => Err(Error {
            span: start,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", other),
                expected: "pattern",
            },
        }),
    }
}

fn parse_field_patterns(cur: &mut Cursor) -> Result<Vec<FieldPat>, Error> {
    let mut fields = Vec::new();
    loop {
        let name_span = cur.peek().span;
        let field = match cur.peek_kind().clone() {
            TokenKind::LowerIdent(n) => {
                cur.bump();
                n
            }
            other => {
                return Err(Error {
                    span: name_span,
                    kind: ErrorKind::Unexpected {
                        found: format!("{:?}", other),
                        expected: "field name",
                    },
                });
            }
        };
        cur.expect(TokenKind::Equals, "`=`")?;
        let pattern = parse_pattern(cur)?;
        fields.push(FieldPat { field, pattern });
        if !cur.eat(&TokenKind::Comma) {
            break;
        }
    }
    Ok(fields)
}

fn starts_pattern(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Underscore
            | TokenKind::LowerIdent(_)
            | TokenKind::UpperIdent(_)
            | TokenKind::IntLit(_)
            | TokenKind::FloatLit(_)
            | TokenKind::StringLit(_)
            | TokenKind::LParen
            | TokenKind::LBracket
    )
}
