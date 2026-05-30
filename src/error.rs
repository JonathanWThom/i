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
    NonExhaustiveMatch {
        missing: Vec<String>,
    },
    CannotAccessMember {
        ty: String,
        member: String,
    },
    MissingImpl {
        trait_name: String,
        ty: String,
    },
    DuplicateImpl {
        trait_name: String,
        ty: String,
    },
    UnknownTrait {
        name: String,
    },
    MissingMethod {
        trait_name: String,
        method: String,
    },
    UnknownMethod {
        trait_name: String,
        method: String,
    },
    AmbiguousConstraint {
        trait_name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trait_error_variants_exist() {
        let _ = ErrorKind::MissingImpl {
            trait_name: "Eq".into(),
            ty: "Point".into(),
        };
        let _ = ErrorKind::DuplicateImpl {
            trait_name: "Eq".into(),
            ty: "Point".into(),
        };
        let _ = ErrorKind::UnknownTrait {
            name: "Nope".into(),
        };
        let _ = ErrorKind::MissingMethod {
            trait_name: "Eq".into(),
            method: "ne".into(),
        };
        let _ = ErrorKind::UnknownMethod {
            trait_name: "Eq".into(),
            method: "zz".into(),
        };
        let _ = ErrorKind::AmbiguousConstraint {
            trait_name: "Eq".into(),
        };
    }
}
