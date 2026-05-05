mod cursor;

use crate::error::Error;
use crate::span::Span;
use crate::token::{Token, TokenKind};
use cursor::Cursor;

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let cur = Cursor::new(src);
    let mut out = Vec::new();
    // Tasks 4-12 fill this in. Today we just emit Eof.
    let span = Span::new(cur.pos(), cur.pos());
    out.push(Token {
        span,
        kind: TokenKind::Eof,
    });
    Ok(out)
}
