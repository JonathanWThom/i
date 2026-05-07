use super::cursor::Cursor;
use crate::ast::{BinOp, Expr, ExprKind, UnaryOp};
use crate::error::{Error, ErrorKind};
use crate::span::Spanned;
use crate::token::TokenKind;

pub(super) fn parse_expr(cur: &mut Cursor) -> Result<Expr, Error> {
    parse_expr_bp(cur, 0)
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
    } else {
        parse_atom(cur)?
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
