use super::types::{ModulePath, Resolution};
use crate::ast::{DeclKind, File};
use crate::error::{Error, ErrorKind};
use crate::span::Span;
use std::collections::{HashMap, HashSet};

pub type ModuleSet = HashMap<ModulePath, File>;
pub type ProjectResolution = HashMap<ModulePath, Resolution>;

pub fn resolve_project(set: &ModuleSet) -> Result<ProjectResolution, Vec<Error>> {
    let cycle_errs = detect_cycles(set);
    if !cycle_errs.is_empty() {
        return Err(cycle_errs);
    }
    let mut out = ProjectResolution::new();
    let mut all_errors = Vec::new();
    for (path, file) in set {
        match super::resolve_file_in_set(file, set) {
            Ok(res) => {
                out.insert(path.clone(), res);
            }
            Err(errs) => all_errors.extend(errs),
        }
    }
    if all_errors.is_empty() {
        Ok(out)
    } else {
        Err(all_errors)
    }
}

fn detect_cycles(set: &ModuleSet) -> Vec<Error> {
    let mut visited: HashSet<ModulePath> = HashSet::new();
    let mut stack: Vec<ModulePath> = Vec::new();
    let mut on_stack: HashSet<ModulePath> = HashSet::new();
    let mut errors: Vec<Error> = Vec::new();
    for path in set.keys() {
        dfs(
            path,
            set,
            &mut visited,
            &mut stack,
            &mut on_stack,
            &mut errors,
        );
    }
    errors
}

fn dfs(
    node: &ModulePath,
    set: &ModuleSet,
    visited: &mut HashSet<ModulePath>,
    stack: &mut Vec<ModulePath>,
    on_stack: &mut HashSet<ModulePath>,
    errors: &mut Vec<Error>,
) {
    if on_stack.contains(node) {
        let cycle_start = stack.iter().position(|m| m == node).unwrap();
        let members = stack[cycle_start..].to_vec();
        errors.push(Error {
            span: Span::new(0, 0),
            kind: ErrorKind::ModuleCycle { members },
        });
        return;
    }
    if visited.contains(node) {
        return;
    }
    let Some(file) = set.get(node) else {
        return;
    };
    stack.push(node.clone());
    on_stack.insert(node.clone());
    for decl in &file.decls {
        if let DeclKind::Use { path, .. } = &decl.node {
            dfs(path, set, visited, stack, on_stack, errors);
        }
    }
    stack.pop();
    on_stack.remove(node);
    visited.insert(node.clone());
}
