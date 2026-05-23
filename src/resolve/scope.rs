use super::module_set::ModuleSet;
use super::types::{DefId, DefInfo, DefKind, LocalId, ModulePath, Resolution};
use crate::ast::{Decl, DeclKind, File, TypeBody, TypeMember, UseKind};
use crate::error::{Error, ErrorKind};
use crate::span::Span;
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct Imports {
    pub modules: Vec<ModulePath>,
    pub cherries: HashMap<String, (ModulePath, String)>,
    #[allow(dead_code)] // populated by Task 13 (alias).
    pub aliases: HashMap<String, ModulePath>,
}

pub(super) fn collect_imports(file: &File, set: &ModuleSet, errors: &mut Vec<Error>) -> Imports {
    let mut imp = Imports::default();
    for decl in &file.decls {
        if let DeclKind::Use { path, kind } = &decl.node {
            if !set.contains_key(path) {
                errors.push(Error {
                    span: decl.span,
                    kind: ErrorKind::UnknownModule { path: path.clone() },
                });
                continue;
            }
            match kind {
                UseKind::Whole => imp.modules.push(path.clone()),
                UseKind::Cherry(names) => {
                    for n in names {
                        imp.cherries.insert(n.clone(), (path.clone(), n.clone()));
                    }
                }
                UseKind::Alias(_) => {
                    // Task 13 fills this in.
                }
            }
        }
    }
    imp
}

#[derive(Default)]
pub(super) struct ScopeStack {
    frames: Vec<Vec<(String, LocalId)>>,
    next_id: u32,
}

impl ScopeStack {
    pub(super) fn new() -> Self {
        Self {
            frames: vec![Vec::new()],
            next_id: 0,
        }
    }

    pub(super) fn push_frame(&mut self) {
        self.frames.push(Vec::new());
    }

    pub(super) fn pop_frame(&mut self) {
        self.frames.pop();
    }

    pub(super) fn push_local(&mut self, name: &str) -> Result<LocalId, ()> {
        let frame = self.frames.last_mut().unwrap();
        if frame.iter().any(|(n, _)| n == name) {
            return Err(());
        }
        let id = LocalId(self.next_id);
        self.next_id += 1;
        frame.push((name.to_string(), id));
        Ok(id)
    }

    pub(super) fn lookup_local(&self, name: &str) -> Option<LocalId> {
        for frame in self.frames.iter().rev() {
            if let Some((_, id)) = frame.iter().find(|(n, _)| n == name) {
                return Some(*id);
            }
        }
        None
    }
}

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
