use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub span: Span,
    pub kind: TokenKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),

    // Identifiers
    LowerIdent(String),
    UpperIdent(String),

    // Keywords (reserved lowercase)
    KwType,
    KwMatch,
    KwModule,
    KwExpose,
    KwUse,
    KwAs,
    KwTrait,
    KwImpl,
    KwAnd,
    KwOr,
    KwNot,
    KwXor,

    // The four binding operators
    Colon,
    Equals,
    Arrow,
    Dot,

    // Punctuation
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Bang,
    Question,
    DotDot,

    // Arithmetic / comparison / concat (desugar in parser)
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    EqEq,
    SlashEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    PlusPlus,

    // Layout (lexer-synthetic)
    Newline,
    Indent,
    Dedent,

    // Sentinel
    Eof,
}

impl TokenKind {
    pub fn is_layout(&self) -> bool {
        matches!(
            self,
            TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
        )
    }
}
