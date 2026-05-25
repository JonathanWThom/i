use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub span: Span,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    UnexpectedChar(char),
    UnderscoreInIdentifier {
        name: String,
        suggestion: String,
    },
    InvalidNumber(String),
    UnterminatedString,
    InvalidEscape(char),
    InconsistentDedent,
    MixedTabsAndSpaces,
    Unexpected {
        found: String,
        expected: &'static str,
    },
    ChainedComparison,
    DuplicateDefinition {
        name: String,
        first_span: Span,
    },
    Unresolved {
        name: String,
    },
    DuplicateLocal {
        name: String,
    },
    UnknownModule {
        path: Vec<String>,
    },
    ModuleCycle {
        members: Vec<Vec<String>>,
    },
    NotExposed {
        module: Vec<String>,
        name: String,
    },
    TypeMismatch {
        expected: String,
        found: String,
    },
    OccursCheck {
        var: String,
    },
    ArityMismatch {
        expected: usize,
        found: usize,
    },
    EffectsNotYetImplemented,
    TuplesNotYetImplemented,
    MixedFieldsAndVariants {
        name: String,
    },
    UnknownType {
        name: String,
    },
    UnknownField {
        type_name: String,
        field: String,
    },
    MissingField {
        type_name: String,
        field: String,
    },
}
