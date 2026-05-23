use super::binding::parse_binding;
use super::cursor::Cursor;
use super::typ;
use super::typedecl::parse_type_decl;
use crate::ast::{Decl, DeclKind, Expose, File, ModuleHeader, UseKind};
use crate::error::{Error, ErrorKind};
use crate::span::Spanned;
use crate::token::TokenKind;

pub(super) fn parse_file(cur: &mut Cursor) -> Result<File, Error> {
    cur.eat(&TokenKind::Newline);
    let module = if cur.check(&TokenKind::KwModule) {
        Some(parse_module_header(cur)?)
    } else {
        None
    };
    let mut decls = Vec::new();
    while !cur.at_end() {
        if cur.eat(&TokenKind::Newline) {
            continue;
        }
        decls.push(parse_decl(cur)?);
    }
    Ok(File { module, decls })
}

fn parse_decl(cur: &mut Cursor) -> Result<Decl, Error> {
    match cur.peek_kind() {
        TokenKind::KwType => parse_type_decl(cur),
        TokenKind::KwTrait => parse_trait_decl(cur),
        TokenKind::KwImpl => parse_impl_decl(cur),
        TokenKind::KwUse => parse_use_decl(cur),
        _ => parse_binding(cur),
    }
}

fn parse_module_header(cur: &mut Cursor) -> Result<ModuleHeader, Error> {
    cur.bump();
    let mut name = vec![expect_upper(cur)?];
    while cur.eat(&TokenKind::Dot) {
        name.push(expect_upper(cur)?);
    }
    cur.expect(TokenKind::Newline, "newline after module name")?;
    cur.expect(TokenKind::Indent, "indented expose clause")?;
    cur.expect(TokenKind::KwExpose, "`expose`")?;
    let exposes = parse_expose_list(cur)?;
    cur.eat(&TokenKind::Newline);
    cur.expect(TokenKind::Dedent, "dedent at module header end")?;
    Ok(ModuleHeader { name, exposes })
}

fn parse_expose_list(cur: &mut Cursor) -> Result<Vec<Expose>, Error> {
    let mut out = vec![parse_expose(cur)?];
    while cur.eat(&TokenKind::Comma) {
        out.push(parse_expose(cur)?);
    }
    Ok(out)
}

fn parse_expose(cur: &mut Cursor) -> Result<Expose, Error> {
    let span = cur.peek().span;
    match cur.peek_kind().clone() {
        TokenKind::LowerIdent(n) => {
            cur.bump();
            Ok(Expose::Value(n))
        }
        TokenKind::UpperIdent(n) => {
            cur.bump();
            let with_constructors = cur.check(&TokenKind::LParen)
                && matches!(cur.peek_n(1), Some(TokenKind::DotDot))
                && matches!(cur.peek_n(2), Some(TokenKind::RParen));
            if with_constructors {
                cur.bump();
                cur.bump();
                cur.bump();
            }
            Ok(Expose::Type {
                name: n,
                with_constructors,
            })
        }
        other => Err(Error {
            span,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", other),
                expected: "exposed item (lowercase value or uppercase type)",
            },
        }),
    }
}

fn parse_use_decl(cur: &mut Cursor) -> Result<Decl, Error> {
    let start = cur.peek().span;
    cur.bump();
    let mut path = vec![expect_upper(cur)?];
    while cur.eat(&TokenKind::Dot) {
        path.push(expect_upper(cur)?);
    }
    let kind = if cur.eat(&TokenKind::KwAs) {
        UseKind::Alias(expect_upper(cur)?)
    } else if cur.eat(&TokenKind::LParen) {
        let mut names = vec![expect_ident(cur)?];
        while cur.eat(&TokenKind::Comma) {
            names.push(expect_ident(cur)?);
        }
        cur.expect(TokenKind::RParen, "`)`")?;
        UseKind::Cherry(names)
    } else {
        UseKind::Whole
    };
    let end = cur.peek().span;
    cur.eat(&TokenKind::Newline);
    Ok(Spanned {
        span: start.merge(end),
        node: DeclKind::Use { path, kind },
    })
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

pub(super) fn expect_upper(cur: &mut Cursor) -> Result<String, Error> {
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

fn expect_ident(cur: &mut Cursor) -> Result<String, Error> {
    let span = cur.peek().span;
    match cur.peek_kind().clone() {
        TokenKind::LowerIdent(n) | TokenKind::UpperIdent(n) => {
            cur.bump();
            Ok(n)
        }
        other => Err(Error {
            span,
            kind: ErrorKind::Unexpected {
                found: format!("{:?}", other),
                expected: "identifier (lower- or uppercase)",
            },
        }),
    }
}

pub(super) fn expect_lower(cur: &mut Cursor) -> Result<String, Error> {
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
