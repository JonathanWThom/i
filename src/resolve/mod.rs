mod scope;
mod types;
mod walker;

pub use types::*;

use crate::ast::File;
use crate::error::Error;

pub fn resolve_file(file: &File) -> Result<Resolution, Vec<Error>> {
    let mut res = Resolution::default();
    let mut errors = scope::collect_top_level(file, &mut res);
    walker::walk_file(file, &mut res, &mut errors);
    if errors.is_empty() {
        Ok(res)
    } else {
        Err(errors)
    }
}
