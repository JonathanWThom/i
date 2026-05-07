use super::cursor::Cursor;
use crate::ast::{Expr, ExprKind};
use crate::error::{Error, ErrorKind};
use crate::span::Spanned;
use crate::token::TokenKind;

pub(super) fn parse_expr(cur: &mut Cursor) -> Result<Expr, Error> {
    parse_atom(cur)
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
