mod scope;
mod types;

pub use types::*;

use crate::ast::File;
use crate::error::Error;

pub fn resolve_file(file: &File) -> Result<Resolution, Vec<Error>> {
    let mut res = Resolution::default();
    let errors = scope::collect_top_level(file, &mut res);
    if errors.is_empty() {
        Ok(res)
    } else {
        Err(errors)
    }
}
