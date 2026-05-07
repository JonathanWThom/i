use super::cursor::Cursor;
use crate::ast::File;
use crate::error::Error;

pub(super) fn parse_file(_cur: &mut Cursor) -> Result<File, Error> {
    Ok(File {
        module: None,
        decls: vec![],
    })
}
