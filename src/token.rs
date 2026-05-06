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
    Underscore,

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

    /// Compact human-readable label for snapshot tests.
    pub fn label(&self) -> String {
        match self {
            TokenKind::IntLit(n) => format!("IntLit {}", n),
            TokenKind::FloatLit(n) => format!("FloatLit {}", n),
            TokenKind::StringLit(s) => format!("StringLit {:?}", s),
            TokenKind::LowerIdent(s) => format!("LowerIdent {:?}", s),
            TokenKind::UpperIdent(s) => format!("UpperIdent {:?}", s),
            other => format!("{:?}", other),
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:<22} @ {:?}", self.kind.label(), self.span)
    }
}
