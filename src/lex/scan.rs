use super::cursor::Cursor;
use crate::error::{Error, ErrorKind};
use crate::span::Span;
use crate::token::TokenKind;

pub(super) fn scan_string(cur: &mut Cursor, start: u32) -> Result<TokenKind, Error> {
    cur.bump(); // opening "
    let mut s = String::new();
    loop {
        match cur.bump() {
            None => {
                return Err(Error {
                    span: Span::new(start, cur.pos()),
                    kind: ErrorKind::UnterminatedString,
                });
            }
            Some(b'"') => break,
            Some(b'\n') => {
                return Err(Error {
                    span: Span::new(start, cur.pos()),
                    kind: ErrorKind::UnterminatedString,
                });
            }
            Some(b'\\') => {
                let esc_start = cur.pos() - 1;
                let c = match cur.bump() {
                    Some(c) => c,
                    None => {
                        return Err(Error {
                            span: Span::new(start, cur.pos()),
                            kind: ErrorKind::UnterminatedString,
                        });
                    }
                };
                let unescaped = match c {
                    b'n' => '\n',
                    b't' => '\t',
                    b'r' => '\r',
                    b'\\' => '\\',
                    b'"' => '"',
                    b'0' => '\0',
                    other => {
                        return Err(Error {
                            span: Span::new(esc_start, cur.pos()),
                            kind: ErrorKind::InvalidEscape(other as char),
                        });
                    }
                };
                s.push(unescaped);
            }
            Some(c) => s.push(c as char),
        }
    }
    Ok(TokenKind::StringLit(s))
}

pub(super) fn scan_number(cur: &mut Cursor, src: &str, start: u32) -> Result<TokenKind, Error> {
    while let Some(c) = cur.peek() {
        if c.is_ascii_digit() {
            cur.bump();
        } else {
            break;
        }
    }
    // Float? Only if `.` is followed by a digit.
    let is_float =
        cur.peek() == Some(b'.') && cur.peek_at(1).map_or(false, |c| c.is_ascii_digit());
    if is_float {
        cur.bump(); // consume '.'
        while let Some(c) = cur.peek() {
            if c.is_ascii_digit() {
                cur.bump();
            } else {
                break;
            }
        }
        let text = &src[start as usize..cur.pos() as usize];
        let n: f64 = text.parse().map_err(|_| Error {
            span: Span::new(start, cur.pos()),
            kind: ErrorKind::InvalidNumber(text.to_string()),
        })?;
        Ok(TokenKind::FloatLit(n))
    } else {
        let text = &src[start as usize..cur.pos() as usize];
        let n: i64 = text.parse().map_err(|_| Error {
            span: Span::new(start, cur.pos()),
            kind: ErrorKind::InvalidNumber(text.to_string()),
        })?;
        Ok(TokenKind::IntLit(n))
    }
}

pub(super) fn scan_underscore(cur: &mut Cursor, src: &str, start: u32) -> Result<TokenKind, Error> {
    cur.bump(); // the underscore itself
    if cur
        .peek()
        .map_or(false, |c| c.is_ascii_alphanumeric() || c == b'_')
    {
        while let Some(c) = cur.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                cur.bump();
            } else {
                break;
            }
        }
        let name = std::str::from_utf8(&src.as_bytes()[start as usize..cur.pos() as usize])
            .unwrap()
            .to_string();
        return Err(Error {
            span: Span::new(start, cur.pos()),
            kind: ErrorKind::UnderscoreInIdentifier {
                suggestion: to_camel_case(&name),
                name,
            },
        });
    }
    Ok(TokenKind::Underscore)
}

pub(super) fn scan_ident_or_keyword(
    cur: &mut Cursor,
    src: &str,
    start: u32,
) -> Result<TokenKind, Error> {
    let first = cur.bump().expect("caller guarantees a leading letter");
    let is_upper = first.is_ascii_uppercase();
    while let Some(c) = cur.peek() {
        if c.is_ascii_alphanumeric() {
            cur.bump();
        } else if c == b'_' {
            // Mid-identifier underscore: scan the rest to report a useful name.
            while let Some(c) = cur.peek() {
                if c.is_ascii_alphanumeric() || c == b'_' {
                    cur.bump();
                } else {
                    break;
                }
            }
            let name = std::str::from_utf8(&src.as_bytes()[start as usize..cur.pos() as usize])
                .unwrap()
                .to_string();
            return Err(Error {
                span: Span::new(start, cur.pos()),
                kind: ErrorKind::UnderscoreInIdentifier {
                    suggestion: to_camel_case(&name),
                    name,
                },
            });
        } else {
            break;
        }
    }
    let text = std::str::from_utf8(&src.as_bytes()[start as usize..cur.pos() as usize])
        .unwrap()
        .to_string();
    Ok(if is_upper {
        TokenKind::UpperIdent(text)
    } else {
        match text.as_str() {
            "type" => TokenKind::KwType,
            "match" => TokenKind::KwMatch,
            "module" => TokenKind::KwModule,
            "expose" => TokenKind::KwExpose,
            "use" => TokenKind::KwUse,
            "as" => TokenKind::KwAs,
            "trait" => TokenKind::KwTrait,
            "impl" => TokenKind::KwImpl,
            "and" => TokenKind::KwAnd,
            "or" => TokenKind::KwOr,
            "not" => TokenKind::KwNot,
            "xor" => TokenKind::KwXor,
            _ => TokenKind::LowerIdent(text),
        }
    })
}

fn to_camel_case(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut upper_next = false;
    for c in snake.chars() {
        if c == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(c.to_uppercase());
            upper_next = false;
        } else {
            out.push(c);
        }
    }
    out
}
