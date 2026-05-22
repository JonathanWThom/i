use super::binding::parse_block;
use super::cursor::Cursor;
use super::decl::{expect_lower, expect_upper};
use super::{expr, typ};
use crate::ast::{Decl, DeclKind, TypeBody, TypeMember, VariantBody};
use crate::error::{Error, ErrorKind};
use crate::span::Spanned;
use crate::token::TokenKind;

pub(super) fn parse_type_decl(cur: &mut Cursor) -> Result<Decl, Error> {
    let start = cur.peek().span;
    cur.bump();
    let name = expect_upper(cur)?;
    let mut params = Vec::new();
    if matches!(cur.peek_kind(), TokenKind::LowerIdent(_)) {
        params.push(expect_lower(cur)?);
        while cur.eat(&TokenKind::Comma) {
            params.push(expect_lower(cur)?);
        }
    }
    if cur.eat(&TokenKind::Equals) {
        let t = typ::parse_type(cur)?;
        let span = start.merge(t.span);
        cur.eat(&TokenKind::Newline);
        return Ok(Spanned {
            span,
            node: DeclKind::TypeDecl {
                name,
                params,
                body: TypeBody::Newtype(t),
            },
        });
    }
    cur.expect(TokenKind::Newline, "newline before type block")?;
    cur.expect(TokenKind::Indent, "indented type block")?;
    let members = parse_type_members(cur)?;
    let end = cur
        .expect(TokenKind::Dedent, "dedent at type block end")?
        .span;
    cur.eat(&TokenKind::Newline);
    Ok(Spanned {
        span: start.merge(end),
        node: DeclKind::TypeDecl {
            name,
            params,
            body: TypeBody::Block(members),
        },
    })
}

fn parse_type_members(cur: &mut Cursor) -> Result<Vec<TypeMember>, Error> {
    let mut members = Vec::new();
    while !cur.check(&TokenKind::Dedent) {
        members.push(parse_type_member(cur)?);
    }
    Ok(members)
}

fn parse_type_member(cur: &mut Cursor) -> Result<TypeMember, Error> {
    let start = cur.peek().span;
    match cur.peek_kind().clone() {
        TokenKind::LowerIdent(n) => {
            cur.bump();
            if cur.eat(&TokenKind::Colon) {
                let t = typ::parse_type(cur)?;
                cur.eat(&TokenKind::Newline);
                Ok(TypeMember::Field { name: n, ty: t })
            } else if cur.eat(&TokenKind::Equals) {
                let value = if cur.check(&TokenKind::Newline) {
                    cur.bump();
                    cur.expect(TokenKind::Indent, "indented method body")?;
                    parse_block(cur)?
                } else {
                    expr::parse_expr(cur)?
                };
                cur.eat(&TokenKind::Newline);
                let span = start.merge(value.span);
                Ok(TypeMember::Method(Spanned {
                    span,
                    node: DeclKind::Binding {
                        name: n,
                        ty: None,
                        value: Some(value),
                    },
                }))
            } else {
                Err(Error {
                    span: cur.peek().span,
                    kind: ErrorKind::Unexpected {
                        found: format!("{:?}", cur.peek_kind()),
                        expected: "`:` or `=` after member name",
                    },
                })
            }
        }
        TokenKind::UpperIdent(n) => {
            cur.bump();
            let body = if cur.eat(&TokenKind::Colon) {
                let t = typ::parse_type(cur)?;
                cur.eat(&TokenKind::Newline);
                VariantBody::Single(t)
            } else if cur.check(&TokenKind::Newline)
                && matches!(cur.peek_n(1), Some(TokenKind::Indent))
            {
                cur.bump();
                cur.bump();
                let fields = parse_type_members(cur)?;
                cur.expect(TokenKind::Dedent, "dedent at variant block end")?;
                cur.eat(&TokenKind::Newline);
                VariantBody::Fields(fields)
            } else {
                cur.eat(&TokenKind::Newline);
                VariantBody::Bare
            };
            Ok(TypeMember::Variant { name: n, body })
        }
        other => Err(Error {
            span: start,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", other),
                expected: "type member (field, method, or variant)",
            },
        }),
    }
}
