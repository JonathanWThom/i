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
            Some(b'_') => {
                cur.bump();
                // If the next char would continue an identifier, this is an
                // underscore-in-identifier error. Otherwise it's the bare
                // wildcard.
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
                    let name =
                        std::str::from_utf8(&src.as_bytes()[start as usize..cur.pos() as usize])
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
                TokenKind::Underscore
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let is_upper = c.is_ascii_uppercase();
                while let Some(c) = cur.peek() {
                    if c.is_ascii_alphanumeric() {
                        cur.bump();
                    } else if c == b'_' {
                        // Identifier with underscore in the middle: scan the
                        // whole would-be name to give a useful error.
                        while let Some(c) = cur.peek() {
                            if c.is_ascii_alphanumeric() || c == b'_' {
                                cur.bump();
                            } else {
                                break;
                            }
                        }
                        let name = std::str::from_utf8(
                            &src.as_bytes()[start as usize..cur.pos() as usize],
                        )
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
                if is_upper {
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

