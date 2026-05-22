use super::cursor::Cursor;
use super::expr::{parse_atom, parse_expr, parse_expr_bp};
use super::pat::parse_pattern;
use crate::ast::{Expr, ExprKind, KwArg, MatchArm};
use crate::error::{Error, ErrorKind};
use crate::span::Spanned;
use crate::token::TokenKind;

pub(super) fn parse_call(cur: &mut Cursor) -> Result<Expr, Error> {
    let func = parse_postfix(cur)?;
    if !starts_call_arg(cur.peek_kind()) {
        return Ok(func);
    }
    let mut args = vec![parse_expr(cur)?];
    while cur.eat(&TokenKind::Comma) {
        args.push(parse_expr(cur)?);
    }
    let span = func.span.merge(args.last().unwrap().span);
    Ok(Spanned {
        span,
        node: ExprKind::Call {
            func: Box::new(func),
            args,
        },
    })
}

fn parse_postfix(cur: &mut Cursor) -> Result<Expr, Error> {
    let mut e = parse_atom(cur)?;
    loop {
        match cur.peek_kind() {
            TokenKind::LParen if looks_like_kwargs(cur) => {
                cur.bump();
                let fields = parse_kwargs(cur)?;
                let close = cur.expect(TokenKind::RParen, "`)`")?.span;
                let span = e.span.merge(close);
                e = match e.node {
                    ExprKind::Ctor(name) => Spanned {
                        span,
                        node: ExprKind::Construct {
                            type_name: name,
                            fields,
                        },
                    },
                    _ => Spanned {
                        span,
                        node: ExprKind::Update {
                            value: Box::new(e),
                            fields,
                        },
                    },
                };
            }
            TokenKind::Dot => {
                cur.bump();
                let next_span = cur.peek().span;
                match cur.peek_kind().clone() {
                    TokenKind::LowerIdent(n) | TokenKind::UpperIdent(n) => {
                        cur.bump();
                        let span = e.span.merge(next_span);
                        e = Spanned {
                            span,
                            node: ExprKind::FieldAccess {
                                receiver: Box::new(e),
                                field: n,
                            },
                        };
                    }
                    other => {
                        return Err(Error {
                            span: next_span,
                            kind: ErrorKind::Unexpected {
                                found: format!("{:?}", other),
                                expected: "field name after `.`",
                            },
                        });
                    }
                }
            }
            TokenKind::Bang => {
                let span = e.span.merge(cur.peek().span);
                cur.bump();
                e = Spanned {
                    span,
                    node: ExprKind::Bang(Box::new(e)),
                };
            }
            TokenKind::Question => {
                let span = e.span.merge(cur.peek().span);
                cur.bump();
                e = Spanned {
                    span,
                    node: ExprKind::Question(Box::new(e)),
                };
            }
            TokenKind::KwMatch => {
                cur.bump();
                cur.expect(TokenKind::Newline, "newline before match arms")?;
                cur.expect(TokenKind::Indent, "indented match arms")?;
                let mut arms = Vec::new();
                while !cur.check(&TokenKind::Dedent) {
                    let pattern = parse_pattern(cur)?;
                    cur.expect(TokenKind::Arrow, "`->`")?;
                    let body = parse_expr_bp(cur, 0)?;
                    cur.eat(&TokenKind::Newline);
                    arms.push(MatchArm { pattern, body });
                }
                let close = cur.expect(TokenKind::Dedent, "dedent")?.span;
                let span = e.span.merge(close);
                e = Spanned {
                    span,
                    node: ExprKind::Match {
                        scrutinee: Box::new(e),
                        arms,
                    },
                };
            }
            _ => break,
        }
    }
    Ok(e)
}

pub(super) fn looks_like_kwargs(cur: &Cursor) -> bool {
    matches!(cur.peek_n(1), Some(TokenKind::LowerIdent(_)))
        && matches!(cur.peek_n(2), Some(TokenKind::Equals))
}

fn parse_kwargs(cur: &mut Cursor) -> Result<Vec<KwArg>, Error> {
    let mut kwargs = Vec::new();
    loop {
        let name_span = cur.peek().span;
        let name = match cur.peek_kind().clone() {
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
        let value = parse_expr_bp(cur, 0)?;
        kwargs.push(KwArg { name, value });
        if !cur.eat(&TokenKind::Comma) {
            break;
        }
    }
    Ok(kwargs)
}

fn starts_call_arg(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::IntLit(_)
            | TokenKind::FloatLit(_)
            | TokenKind::StringLit(_)
            | TokenKind::LowerIdent(_)
            | TokenKind::UpperIdent(_)
            | TokenKind::LParen
            | TokenKind::LBracket
    )
}
