use super::types::{DefId, DefInfo, DefKind, Resolution};
use crate::ast::{Decl, DeclKind, File};
use crate::error::Error;
use crate::span::Span;

pub(super) fn collect_top_level(file: &File, res: &mut Resolution) -> Vec<Error> {
    let errors = Vec::new();
    for decl in &file.decls {
        collect_decl(decl, res);
    }
    errors
}

fn collect_decl(decl: &Decl, res: &mut Resolution) {
    match &decl.node {
        DeclKind::Binding { name, .. } => push_def(res, name.clone(), DefKind::Value, decl.span),
        DeclKind::TypeDecl { name, .. } => push_def(res, name.clone(), DefKind::Type, decl.span),
        DeclKind::TraitDecl { name, .. } => push_def(res, name.clone(), DefKind::Trait, decl.span),
        DeclKind::ImplDecl { .. } | DeclKind::Use { .. } => {}
    }
}

fn push_def(res: &mut Resolution, name: String, kind: DefKind, span: Span) {
    let id = DefId(res.defs.len() as u32);
    res.defs.push(DefInfo {
        id,
        name,
        kind,
        span,
    });
}
