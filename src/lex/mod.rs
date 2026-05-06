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

        // Newline handling. A `\n` may emit a Newline token, then read the
        // indent of the next significant line and emit Indent/Dedent(s) as
        // appropriate. Inside parens or after a continuation operator,
        // layout is suppressed entirely.
        if cur.peek() == Some(b'\n') {
            let nl_start = cur.pos();
            cur.bump();
            let suppress = layout.suppresses_newline();
            let in_parens = layout.paren_depth > 0;
            let layout_active = !suppress && !in_parens;

            if layout_active {
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

            // Read leading whitespace of the next significant line. Blank
            // and comment-only lines are skipped.
            match read_indent_or_eof(&mut cur) {
                None => break,
                Some(col) => {
                    if layout_active {
                        let pos = cur.pos();
                        while col < layout.top() {
                            layout.indent_stack.pop();
                            let kind = TokenKind::Dedent;
                            out.push(Token {
                                span: Span::new(pos, pos),
                                kind: kind.clone(),
                            });
                            layout.note_emitted(&kind);
                        }
                        if col > layout.top() {
                            layout.indent_stack.push(col);
                            let kind = TokenKind::Indent;
                            out.push(Token {
                                span: Span::new(pos, pos),
                                kind: kind.clone(),
                            });
                            layout.note_emitted(&kind);
                        } else if col != layout.top() {
                            return Err(Error {
                                span: Span::new(pos, pos),
                                kind: ErrorKind::InconsistentDedent,
                            });
                        }
                    }
                    continue;
                }
            }
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

    // Drain any remaining open blocks at EOF.
    let end = cur.pos();
    while layout.top() > 0 {
        layout.indent_stack.pop();
        let kind = TokenKind::Dedent;
        out.push(Token {
            span: Span::new(end, end),
            kind: kind.clone(),
        });
        layout.note_emitted(&kind);
    }

    out.push(Token {
        span: Span::new(end, end),
        kind: TokenKind::Eof,
    });
    Ok(out)
}

/// Reads leading whitespace of the next significant (non-blank,
/// non-comment-only) line, skipping any number of blank/comment-only
/// lines along the way. Returns the column of the first content byte,
/// or None at EOF.
fn read_indent_or_eof(cur: &mut Cursor) -> Option<u32> {
    loop {
        let mut col = 0u32;
        while let Some(c) = cur.peek() {
            match c {
                b' ' | b'\t' => {
                    cur.bump();
                    col += 1;
                }
                _ => break,
            }
        }
        match cur.peek() {
            None => return None,
            Some(b'\n') => {
                cur.bump();
                continue;
            }
            Some(b'#') => {
                while let Some(c) = cur.peek() {
                    if c == b'\n' {
                        break;
                    }
                    cur.bump();
                }
                continue;
            }
            _ => return Some(col),
        }
    }
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
