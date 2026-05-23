pub mod types;
pub mod unify;

pub use types::*;

use crate::ast::File;
use crate::error::Error;
use crate::resolve::Resolution;

pub fn check_file(_file: &File, _res: &Resolution) -> Result<Typing, Vec<Error>> {
    Ok(Typing::default())
}
