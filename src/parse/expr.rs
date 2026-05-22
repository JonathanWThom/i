use super::cursor::Cursor;
use super::pat::parse_pattern;
use crate::ast::{BinOp, Expr, ExprKind, KwArg, MatchArm, Pattern, UnaryOp};
use crate::error::{Error, ErrorKind};
use crate::span::Spanned;
use crate::token::TokenKind;

pub(super) fn parse_expr(cur: &mut Cursor) -> Result<Expr, Error> {
    if looks_like_lambda(cur) {
        return parse_lambda(cur);
    }
    parse_expr_bp(cur, 0)
}

fn looks_like_lambda(cur: &Cursor) -> bool {
    let mut i = 0;
    loop {
        match cur.peek_n(i) {
            Some(TokenKind::LowerIdent(_)) => i += 1,
            Some(TokenKind::Arrow) if i > 0 => return true,
            _ => return false,
        }
    }
}

fn parse_lambda(cur: &mut Cursor) -> Result<Expr, Error> {
    let start = cur.peek().span;
    let mut params: Vec<Pattern> = Vec::new();
    while !cur.check(&TokenKind::Arrow) {
        params.push(parse_pattern(cur)?);
    }
    cur.bump();
    let body = parse_expr_bp(cur, 0)?;
    let span = start.merge(body.span);
    Ok(Spanned {
        span,
        node: ExprKind::Lambda {
            params,
            body: Box::new(body),
        },
    })
}

fn parse_expr_bp(cur: &mut Cursor, min_bp: u8) -> Result<Expr, Error> {
    let mut lhs = if cur.check(&TokenKind::Minus) {
        let start = cur.peek().span;
        cur.bump();
        let rhs = parse_expr_bp(cur, 100)?;
        let span = start.merge(rhs.span);
        Spanned {
            span,
            node: ExprKind::UnaryOp {
                op: UnaryOp::Neg,
                expr: Box::new(rhs),
            },
        }
    } else if cur.check(&TokenKind::KwNot) {
        let start = cur.peek().span;
        cur.bump();
        let rhs = parse_expr_bp(cur, 40)?;
        let span = start.merge(rhs.span);
        Spanned {
            span,
            node: ExprKind::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(rhs),
            },
        }
    } else {
        parse_call(cur)?
    };

    while let Some((op, lbp, rbp)) = infix_for(cur.peek_kind()) {
        if lbp < min_bp {
            break;
        }

        if is_comparison(&op) {
            cur.bump();
            let rhs = parse_expr_bp(cur, rbp)?;
            let span = lhs.span.merge(rhs.span);
            if let Some((next_op, _, _)) = infix_for(cur.peek_kind())
                && is_comparison(&next_op)
            {
                return Err(Error {
                    span: cur.peek().span,
                    kind: ErrorKind::ChainedComparison,
                });
            }
            lhs = Spanned {
                span,
                node: ExprKind::BinOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            };
            continue;
        }

        cur.bump();
        let rhs = parse_expr_bp(cur, rbp)?;
        let span = lhs.span.merge(rhs.span);
        lhs = Spanned {
            span,
            node: ExprKind::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
        };
    }

    Ok(lhs)
}

fn is_comparison(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    )
}

fn infix_for(k: &TokenKind) -> Option<(BinOp, u8, u8)> {
    Some(match k {
        TokenKind::KwOr => (BinOp::Or, 20, 21),
        TokenKind::KwAnd => (BinOp::And, 30, 31),
        TokenKind::EqEq => (BinOp::Eq, 50, 51),
        TokenKind::SlashEq => (BinOp::Ne, 50, 51),
        TokenKind::Lt => (BinOp::Lt, 50, 51),
        TokenKind::LtEq => (BinOp::Le, 50, 51),
        TokenKind::Gt => (BinOp::Gt, 50, 51),
        TokenKind::GtEq => (BinOp::Ge, 50, 51),
        TokenKind::PlusPlus => (BinOp::Concat, 60, 60),
        TokenKind::Plus => (BinOp::Add, 70, 71),
        TokenKind::Minus => (BinOp::Sub, 70, 71),
        TokenKind::Star => (BinOp::Mul, 80, 81),
        TokenKind::Slash => (BinOp::Div, 80, 81),
        TokenKind::Caret => (BinOp::Pow, 90, 90),
        _ => return None,
    })
}

fn parse_call(cur: &mut Cursor) -> Result<Expr, Error> {
    let func = parse_postfix(cur)?;
    if !starts_call_arg(cur.peek_kind()) {
        return Ok(func);
    }
    let mut args = vec![parse_expr_bp(cur, 0)?];
    while cur.eat(&TokenKind::Comma) {
        args.push(parse_expr_bp(cur, 0)?);
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

pub(super) fn parse_atom(cur: &mut Cursor) -> Result<Expr, Error> {
    let start = cur.peek().span;
    match cur.peek_kind().clone() {
        TokenKind::IntLit(n) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: ExprKind::IntLit(n),
            })
        }
        TokenKind::FloatLit(n) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: ExprKind::FloatLit(n),
            })
        }
        TokenKind::StringLit(s) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: ExprKind::StringLit(s),
            })
        }
        TokenKind::LowerIdent(s) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: ExprKind::Var(s),
            })
        }
        TokenKind::UpperIdent(s) => {
            cur.bump();
            Ok(Spanned {
                span: start,
                node: ExprKind::Ctor(s),
            })
        }
        TokenKind::LParen => {
            cur.bump();
            let e = parse_expr(cur)?;
            let close = cur.expect(TokenKind::RParen, "`)`")?.span;
            Ok(Spanned {
                span: start.merge(close),
                node: ExprKind::Paren(Box::new(e)),
            })
        }
        TokenKind::LBracket => {
            cur.bump();
            let mut items = Vec::new();
            if !cur.check(&TokenKind::RBracket) {
                items.push(parse_expr(cur)?);
                while cur.eat(&TokenKind::Comma) {
                    items.push(parse_expr(cur)?);
                }
            }
            let close = cur.expect(TokenKind::RBracket, "`]`")?.span;
            Ok(Spanned {
                span: start.merge(close),
                node: ExprKind::List(items),
            })
        }
        _ => Err(Error {
            span: start,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", cur.peek_kind()),
                expected: "an expression",
            },
        }),
    }
}
