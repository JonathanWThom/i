mod module_set;
mod scope;
mod types;
mod walker;

pub use module_set::{ModuleSet, ProjectResolution, resolve_project};
pub use types::*;

use crate::ast::File;
use crate::error::Error;
use std::collections::HashMap;

pub fn resolve_file(file: &File) -> Result<Resolution, Vec<Error>> {
    resolve_file_in_set(file, &HashMap::new())
}

pub(crate) fn resolve_file_in_set(file: &File, set: &ModuleSet) -> Result<Resolution, Vec<Error>> {
    let mut res = Resolution::default();
    let mut errors = scope::collect_top_level(file, &mut res);
    let imports = scope::collect_imports(file, set, &mut errors);
    walker::walk_file(file, &mut res, &mut errors, &imports);
    if errors.is_empty() {
        Ok(res)
    } else {
        Err(errors)
    }
}
