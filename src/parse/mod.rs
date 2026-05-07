mod cursor;
mod decl;
mod expr;
mod pat;
mod typ;

use crate::ast::{Expr, File};
use crate::error::Error;
use crate::token::Token;
use cursor::Cursor;

pub fn parse(toks: &[Token]) -> Result<File, Error> {
    let mut cur = Cursor::new(toks);
    decl::parse_file(&mut cur)
}

#[doc(hidden)]
pub fn parse_expr_for_test(toks: &[Token]) -> Result<Expr, Error> {
    let mut cur = Cursor::new(toks);
    expr::parse_expr(&mut cur)
}
