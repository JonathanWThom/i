use super::cursor::Cursor;
use super::{expr, typ};
use crate::ast::{BlockItem, Decl, DeclKind, Expr, ExprKind};
use crate::error::{Error, ErrorKind};
use crate::span::Spanned;
use crate::token::TokenKind;

pub(super) fn parse_binding(cur: &mut Cursor) -> Result<Decl, Error> {
    let start = cur.peek().span;
    let name = match cur.peek_kind().clone() {
        TokenKind::LowerIdent(n) => {
            cur.bump();
            n
        }
        other => {
            return Err(Error {
                span: start,
                kind: ErrorKind::Unexpected {
                    found: format!("{:?}", other),
                    expected: "binding name",
                },
            });
        }
    };

    let ty = if cur.eat(&TokenKind::Colon) {
        Some(typ::parse_type(cur)?)
    } else {
        None
    };

    let value = if cur.eat(&TokenKind::Equals) {
        if cur.check(&TokenKind::Newline) {
            cur.bump();
            cur.expect(TokenKind::Indent, "indented block body")?;
            Some(parse_block(cur)?)
        } else {
            Some(expr::parse_expr(cur)?)
        }
    } else {
        None
    };

    cur.eat(&TokenKind::Newline);

    let end_span = match (&ty, &value) {
        (_, Some(v)) => v.span,
        (Some(t), None) => t.span,
        (None, None) => start,
    };
    Ok(Spanned {
        span: start.merge(end_span),
        node: DeclKind::Binding { name, ty, value },
    })
}

pub(super) fn parse_block(cur: &mut Cursor) -> Result<Expr, Error> {
    let start = cur.peek().span;
    let mut items = Vec::new();
    while !cur.check(&TokenKind::Dedent) {
        if looks_like_binding(cur) {
            items.push(BlockItem::Binding(parse_binding(cur)?));
        } else {
            let e = expr::parse_expr(cur)?;
            cur.eat(&TokenKind::Newline);
            items.push(BlockItem::Expr(e));
        }
    }
    let close = cur.expect(TokenKind::Dedent, "dedent at block end")?.span;
    Ok(Spanned {
        span: start.merge(close),
        node: ExprKind::Block(items),
    })
}

fn looks_like_binding(cur: &Cursor) -> bool {
    matches!(cur.peek_kind(), TokenKind::LowerIdent(_))
        && matches!(
            cur.peek_n(1),
            Some(TokenKind::Colon) | Some(TokenKind::Equals)
        )
}
