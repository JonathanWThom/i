use super::types::{DefId, DefInfo, DefKind, Resolution};
use crate::ast::{Decl, DeclKind, File, TypeBody, TypeMember};
use crate::error::{Error, ErrorKind};
use crate::span::Span;
use std::collections::HashMap;

pub(super) fn collect_top_level(file: &File, res: &mut Resolution) -> Vec<Error> {
    let mut errors = Vec::new();
    let mut value_seen: HashMap<String, Span> = HashMap::new();
    let mut type_seen: HashMap<String, Span> = HashMap::new();
    for decl in &file.decls {
        collect_decl(decl, res, &mut value_seen, &mut type_seen, &mut errors);
    }
    errors
}

fn collect_decl(
    decl: &Decl,
    res: &mut Resolution,
    value_seen: &mut HashMap<String, Span>,
    type_seen: &mut HashMap<String, Span>,
    errors: &mut Vec<Error>,
) {
    match &decl.node {
        DeclKind::Binding { name, value, .. } => {
            if value.is_some() {
                check_dup(name, decl.span, value_seen, errors);
                push_def(res, name.clone(), DefKind::Value, decl.span);
            }
        }
        DeclKind::TypeDecl { name, body, .. } => {
            check_dup(name, decl.span, type_seen, errors);
            let type_id = DefId(res.defs.len() as u32);
            push_def(res, name.clone(), DefKind::Type, decl.span);
            if let TypeBody::Block(members) = body {
                for m in members {
                    if let TypeMember::Variant { name: vname, .. } = m {
                        check_dup(vname, decl.span, value_seen, errors);
                        let id = DefId(res.defs.len() as u32);
                        res.defs.push(DefInfo {
                            id,
                            name: vname.clone(),
                            kind: DefKind::Ctor { of_type: type_id },
                            span: decl.span,
                        });
                    }
                }
            }
        }
        DeclKind::TraitDecl { name, .. } => {
            check_dup(name, decl.span, type_seen, errors);
            push_def(res, name.clone(), DefKind::Trait, decl.span);
        }
        DeclKind::ImplDecl { .. } | DeclKind::Use { .. } => {}
    }
}

fn check_dup(name: &str, span: Span, seen: &mut HashMap<String, Span>, errors: &mut Vec<Error>) {
    if let Some(first) = seen.get(name) {
        errors.push(Error {
            span,
            kind: ErrorKind::DuplicateDefinition {
                name: name.to_string(),
                first_span: *first,
            },
        });
    } else {
        seen.insert(name.to_string(), span);
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
