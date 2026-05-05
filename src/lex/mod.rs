mod cursor;

use crate::error::{Error, ErrorKind};
use crate::span::Span;
use crate::token::{Token, TokenKind};
use cursor::Cursor;

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let mut cur = Cursor::new(src);
    let mut out = Vec::new();

    loop {
        // Skip horizontal whitespace within a line.
        while let Some(b' ') = cur.peek() {
            cur.bump();
        }
        let start = cur.pos();
        let kind = match cur.peek() {
            None => break,
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
