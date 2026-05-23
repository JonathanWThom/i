use super::types::{ModulePath, Resolution};
use crate::ast::File;
use crate::error::Error;
use std::collections::HashMap;

pub type ModuleSet = HashMap<ModulePath, File>;
pub type ProjectResolution = HashMap<ModulePath, Resolution>;

pub fn resolve_project(set: &ModuleSet) -> Result<ProjectResolution, Vec<Error>> {
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
