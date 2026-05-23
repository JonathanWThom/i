use super::types::{DefKind, Resolution, ResolvedName};
use crate::ast::{Decl, DeclKind, Expr, ExprKind, File};
use crate::error::{Error, ErrorKind};
use crate::span::Span;

pub(super) fn walk_file(file: &File, res: &mut Resolution, errors: &mut Vec<Error>) {
    for decl in &file.decls {
        walk_decl(decl, res, errors);
    }
}

fn walk_decl(decl: &Decl, res: &mut Resolution, errors: &mut Vec<Error>) {
    if let DeclKind::Binding { value: Some(v), .. } = &decl.node {
        walk_expr(v, res, errors);
    }
}

fn walk_expr(e: &Expr, res: &mut Resolution, errors: &mut Vec<Error>) {
    match &e.node {
        ExprKind::Var(name) => resolve_var(name, e.span, res, errors),
        ExprKind::IntLit(_) | ExprKind::FloatLit(_) | ExprKind::StringLit(_) => {}
        ExprKind::Ctor(name) => resolve_ctor(name, e.span, res, errors),
        ExprKind::Paren(inner) => walk_expr(inner, res, errors),
        ExprKind::BinOp { lhs, rhs, .. } => {
            walk_expr(lhs, res, errors);
            walk_expr(rhs, res, errors);
        }
        ExprKind::UnaryOp { expr, .. } => walk_expr(expr, res, errors),
        ExprKind::List(items) => items.iter().for_each(|i| walk_expr(i, res, errors)),
        ExprKind::Construct { type_name, fields } => {
            resolve_type_or_ctor(type_name, e.span, res, errors);
            for kw in fields {
                walk_expr(&kw.value, res, errors);
            }
        }
        ExprKind::Update { value, fields } => {
            walk_expr(value, res, errors);
            for kw in fields {
                walk_expr(&kw.value, res, errors);
            }
        }
        _ => {}
    }
}

fn resolve_var(name: &str, span: Span, res: &mut Resolution, errors: &mut Vec<Error>) {
    if let Some(def) = res
        .defs
        .iter()
        .find(|d| d.name == name && matches!(d.kind, DefKind::Value))
    {
        res.refs.insert(span, ResolvedName::TopLevel(def.id));
    } else {
        errors.push(Error {
            span,
            kind: ErrorKind::Unresolved {
                name: name.to_string(),
            },
        });
    }
}

fn resolve_ctor(name: &str, span: Span, res: &mut Resolution, errors: &mut Vec<Error>) {
    if let Some(def) = res
        .defs
        .iter()
        .find(|d| d.name == name && matches!(d.kind, DefKind::Ctor { .. }))
    {
        res.refs.insert(span, ResolvedName::Ctor(def.id));
    } else {
        errors.push(Error {
            span,
            kind: ErrorKind::Unresolved {
                name: name.to_string(),
            },
        });
    }
}

fn resolve_type_or_ctor(name: &str, span: Span, res: &mut Resolution, errors: &mut Vec<Error>) {
    if let Some(def) = res.defs.iter().find(|d| d.name == name) {
        let resolved = match def.kind {
            DefKind::Ctor { .. } => ResolvedName::Ctor(def.id),
            _ => ResolvedName::TopLevel(def.id),
        };
        res.refs.insert(span, resolved);
    } else {
        errors.push(Error {
            span,
            kind: ErrorKind::Unresolved {
                name: name.to_string(),
            },
        });
    }
}
