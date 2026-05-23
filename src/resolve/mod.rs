mod types;

pub use types::*;

use crate::ast::{DeclKind, File};
use crate::error::Error;

pub fn resolve_file(file: &File) -> Result<Resolution, Vec<Error>> {
    let mut res = Resolution::default();
    if let Some(decl) = file.decls.first()
        && let DeclKind::Binding { name, .. } = &decl.node
    {
        res.defs.push(DefInfo {
            id: DefId(0),
            name: name.clone(),
            kind: DefKind::Value,
            span: decl.span,
        });
    }
    Ok(res)
}
