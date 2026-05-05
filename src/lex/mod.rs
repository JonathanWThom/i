mod cursor;
mod layout;
mod scan;

use crate::error::{Error, ErrorKind};
use crate::span::Span;
use crate::token::{Token, TokenKind};
use cursor::Cursor;
use layout::Layout;

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let mut cur = Cursor::new(src);
    let mut out = Vec::new();
    let mut layout = Layout::new();

    loop {
        skip_trivia(&mut cur);

        // Newline handling. A `\n` either:
        //   - is suppressed (inside parens or after a continuation operator),
        //   - or emits a Newline token (collapsing consecutive blanks),
        // and then we continue scanning the next line.
        if cur.peek() == Some(b'\n') {
            let nl_start = cur.pos();
            cur.bump();
            if !layout.suppresses_newline() {
                let last_was_newline = matches!(
                    out.last().map(|t: &Token| &t.kind),
                    Some(TokenKind::Newline)
                );
                if !last_was_newline && !out.is_empty() {
                    let kind = TokenKind::Newline;
                    out.push(Token {
                        span: Span::new(nl_start, cur.pos()),
                        kind: kind.clone(),
                    });
                    layout.note_emitted(&kind);
                }
            }
            continue;
        }

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
        out.push(Token {
            span,
            kind: kind.clone(),
        });
        layout.note_emitted(&kind);
    }

    let end = cur.pos();
    out.push(Token {
        span: Span::new(end, end),
        kind: TokenKind::Eof,
    });
    Ok(out)
}

/// Skips spaces, tabs, and `#` line comments. Newlines are NOT consumed
/// here — `lex()` handles them so it can decide whether to emit a Newline
/// token (per `Layout::suppresses_newline`).
fn skip_trivia(cur: &mut Cursor) {
    loop {
        match cur.peek() {
            Some(b' ') | Some(b'\t') => {
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
