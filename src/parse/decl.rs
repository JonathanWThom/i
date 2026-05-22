use super::cursor::Cursor;
use super::{expr, typ};
use crate::ast::{
    BlockItem, Decl, DeclKind, Expr, ExprKind, File, TypeBody, TypeMember, VariantBody,
};
use crate::error::{Error, ErrorKind};
use crate::span::Spanned;
use crate::token::TokenKind;

pub(super) fn parse_file(cur: &mut Cursor) -> Result<File, Error> {
    let mut decls = Vec::new();
    cur.eat(&TokenKind::Newline);
    while !cur.at_end() {
        decls.push(parse_decl(cur)?);
    }
    Ok(File {
        module: None,
        decls,
    })
}

fn parse_decl(cur: &mut Cursor) -> Result<Decl, Error> {
    match cur.peek_kind() {
        TokenKind::KwType => parse_type_decl(cur),
        TokenKind::KwTrait => parse_trait_decl(cur),
        TokenKind::KwImpl => parse_impl_decl(cur),
        _ => parse_binding(cur),
    }
}

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

fn parse_block(cur: &mut Cursor) -> Result<Expr, Error> {
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

fn parse_type_decl(cur: &mut Cursor) -> Result<Decl, Error> {
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

fn parse_trait_decl(cur: &mut Cursor) -> Result<Decl, Error> {
    let start = cur.peek().span;
    cur.bump();
    let name = expect_upper(cur)?;
    let type_var = expect_lower(cur)?;
    cur.expect(TokenKind::Newline, "newline before trait block")?;
    cur.expect(TokenKind::Indent, "indented trait block")?;
    let mut methods = Vec::new();
    while !cur.check(&TokenKind::Dedent) {
        methods.push(parse_binding(cur)?);
    }
    let end = cur.expect(TokenKind::Dedent, "dedent at trait end")?.span;
    cur.eat(&TokenKind::Newline);
    Ok(Spanned {
        span: start.merge(end),
        node: DeclKind::TraitDecl {
            name,
            type_var,
            methods,
        },
    })
}

fn parse_impl_decl(cur: &mut Cursor) -> Result<Decl, Error> {
    let start = cur.peek().span;
    cur.bump();
    let trait_name = expect_upper(cur)?;
    let target = typ::parse_type(cur)?;
    cur.expect(TokenKind::Newline, "newline before impl block")?;
    cur.expect(TokenKind::Indent, "indented impl block")?;
    let mut methods = Vec::new();
    while !cur.check(&TokenKind::Dedent) {
        methods.push(parse_binding(cur)?);
    }
    let end = cur.expect(TokenKind::Dedent, "dedent at impl end")?.span;
    cur.eat(&TokenKind::Newline);
    Ok(Spanned {
        span: start.merge(end),
        node: DeclKind::ImplDecl {
            trait_name,
            target,
            methods,
        },
    })
}

fn expect_upper(cur: &mut Cursor) -> Result<String, Error> {
    let span = cur.peek().span;
    match cur.peek_kind().clone() {
        TokenKind::UpperIdent(n) => {
            cur.bump();
            Ok(n)
        }
        other => Err(Error {
            span,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", other),
                expected: "uppercase identifier",
            },
        }),
    }
}

fn expect_lower(cur: &mut Cursor) -> Result<String, Error> {
    let span = cur.peek().span;
    match cur.peek_kind().clone() {
        TokenKind::LowerIdent(n) => {
            cur.bump();
            Ok(n)
        }
        other => Err(Error {
            span,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", other),
                expected: "lowercase identifier",
            },
        }),
    }
}
