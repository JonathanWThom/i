mod cursor;
mod scan;

use crate::error::{Error, ErrorKind};
use crate::span::Span;
use crate::token::{Token, TokenKind};
use cursor::Cursor;

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let mut cur = Cursor::new(src);
    let mut out = Vec::new();

    loop {
        skip_trivia(&mut cur);
        let start = cur.pos();
        let kind = match cur.peek() {
            None => break,

            // Multi-character scanners
            Some(b'"') => scan::scan_string(&mut cur, start)?,
            Some(c) if c.is_ascii_digit() => scan::scan_number(&mut cur, src, start)?,
            Some(b'_') => scan::scan_underscore(&mut cur, src, start)?,
            Some(c) if c.is_ascii_alphabetic() => {
                scan::scan_ident_or_keyword(&mut cur, src, start)?
            }

            // Single- and two-char punctuation, inline.
            Some(b'(') => {
                cur.bump();
                TokenKind::LParen
            }
            Some(b')') => {
                cur.bump();
                TokenKind::RParen
            }
            Some(b'[') => {
                cur.bump();
                TokenKind::LBracket
            }
            Some(b']') => {
                cur.bump();
                TokenKind::RBracket
            }
            Some(b',') => {
                cur.bump();
                TokenKind::Comma
            }
            Some(b'+') => {
                cur.bump();
                if cur.bump_if(b'+') {
                    TokenKind::PlusPlus
                } else {
                    TokenKind::Plus
                }
            }
            Some(b'-') => {
                cur.bump();
                if cur.bump_if(b'>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            Some(b'*') => {
                cur.bump();
                TokenKind::Star
            }
            Some(b'/') => {
                cur.bump();
                if cur.bump_if(b'=') {
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            Some(b'^') => {
                cur.bump();
                TokenKind::Caret
            }
            Some(b'=') => {
                cur.bump();
                if cur.bump_if(b'=') {
                    TokenKind::EqEq
                } else {
                    TokenKind::Equals
                }
            }
            Some(b'<') => {
                cur.bump();
                if cur.bump_if(b'=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            Some(b'>') => {
                cur.bump();
                if cur.bump_if(b'=') {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            Some(b'!') => {
                cur.bump();
                TokenKind::Bang
            }
            Some(b'?') => {
                cur.bump();
                TokenKind::Question
            }
            Some(b':') => {
                cur.bump();
                TokenKind::Colon
            }
            Some(b'.') => {
                cur.bump();
                if cur.bump_if(b'.') {
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }

            Some(c) => {
                return Err(Error {
                    span: Span::new(start, start + 1),
                    kind: ErrorKind::UnexpectedChar(c as char),
                });
            }
        };
        let span = Span::new(start, cur.pos());
        out.push(Token { span, kind });
    }

    let end = cur.pos();
    out.push(Token {
        span: Span::new(end, end),
        kind: TokenKind::Eof,
    });
    Ok(out)
}

// Whitespace and line comments. Newlines are eaten as trivia here too;
// Task 9 replaces the newline arm with proper layout-token emission.
fn skip_trivia(cur: &mut Cursor) {
    loop {
        match cur.peek() {
            Some(b' ') | Some(b'\t') | Some(b'\n') => {
                cur.bump();
            }
            Some(b'#') => {
                while let Some(c) = cur.peek() {
                    if c == b'\n' {
                        break;
                    }
                    cur.bump();
                }
            }
            _ => break,
        }
    }
}
