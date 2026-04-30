use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub span: Span,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    // populated by later tasks
}
