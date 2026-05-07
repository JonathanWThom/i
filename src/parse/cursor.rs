use crate::error::{Error, ErrorKind};
use crate::token::{Token, TokenKind};

pub(super) struct Cursor<'a> {
    toks: &'a [Token],
    pos: usize,
}

#[allow(dead_code)]
impl<'a> Cursor<'a> {
    pub fn new(toks: &'a [Token]) -> Self {
        Self { toks, pos: 0 }
    }

    pub fn peek(&self) -> &Token {
        &self.toks[self.pos]
    }

    pub fn peek_kind(&self) -> &TokenKind {
        &self.toks[self.pos].kind
    }

    pub fn at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    pub fn bump(&mut self) -> &Token {
        let t = &self.toks[self.pos];
        if !self.at_end() {
            self.pos += 1;
        }
        t
    }

    pub fn check(&self, k: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(k)
    }

    pub fn eat(&mut self, k: &TokenKind) -> bool {
        if self.check(k) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub fn expect(&mut self, k: TokenKind, expected: &'static str) -> Result<&Token, Error> {
        if self.check(&k) {
            Ok(self.bump())
        } else {
            let span = self.peek().span;
            Err(Error {
                span,
                kind: ErrorKind::Unexpected {
                    found: format!("{:?}", self.peek_kind()),
                    expected,
                },
            })
        }
    }
}
