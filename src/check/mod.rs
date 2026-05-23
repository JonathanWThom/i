pub mod infer;
pub mod types;
pub mod unify;

pub use types::*;

use crate::ast::{DeclKind, File};
use crate::check::infer::Infer;
use crate::error::Error;
use crate::resolve::Resolution;

pub fn check_file(file: &File, res: &Resolution) -> Result<Typing, Vec<Error>> {
    let mut infer = Infer::new();
    for decl in &file.decls {
        if let DeclKind::Binding {
            name,
            value: Some(value),
            ..
        } = &decl.node
        {
            let Some(def) = res.defs.iter().find(|d| &d.name == name) else {
                continue;
            };
            let ty = infer.infer_expr(value);
            infer.schemes.insert(
                def.id,
                Scheme {
                    vars: Vec::new(),
                    ty,
                },
            );
        }
    }
    if infer.errors.is_empty() {
        Ok(infer.into_typing())
    } else {
        let errs = std::mem::take(&mut infer.errors);
        Err(errs)
    }
}
