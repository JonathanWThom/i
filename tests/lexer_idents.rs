use i_lang::error::ErrorKind;
use i_lang::lex::lex;
use i_lang::token::TokenKind::{self, *};

fn kinds(src: &str) -> Vec<TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn lower_ident() {
    assert_eq!(
        kinds("foo barBaz x1"),
        vec![
            LowerIdent("foo".into()),
            LowerIdent("barBaz".into()),
            LowerIdent("x1".into()),
            Eof,
        ]
    );
}

#[test]
fn upper_ident() {
    assert_eq!(
        kinds("Point List Maybe"),
        vec![
            UpperIdent("Point".into()),
            UpperIdent("List".into()),
            UpperIdent("Maybe".into()),
            Eof,
        ]
    );
}

#[test]
fn keywords() {
    assert_eq!(
        kinds("type match module expose use as trait impl and or not xor"),
        vec![
            KwType, KwMatch, KwModule, KwExpose, KwUse, KwAs, KwTrait, KwImpl, KwAnd, KwOr,
            KwNot, KwXor, Eof,
        ]
    );
}

#[test]
fn keyword_lookalike_is_ident() {
    assert_eq!(
        kinds("types typeof"),
        vec![
            LowerIdent("types".into()),
            LowerIdent("typeof".into()),
            Eof,
        ]
    );
}

#[test]
fn bare_underscore_is_wildcard() {
    assert_eq!(kinds("_"), vec![Underscore, Eof]);
}

#[test]
fn underscore_then_punct_is_wildcard() {
    // `_` followed by non-identifier char is just the wildcard token.
    assert_eq!(
        kinds("_ , _ ->"),
        vec![Underscore, Comma, Underscore, Arrow, Eof]
    );
}

#[test]
fn leading_underscore_is_error() {
    let err = lex("_foo").unwrap_err();
    match err.kind {
        ErrorKind::UnderscoreInIdentifier { name, suggestion } => {
            assert_eq!(name, "_foo");
            assert_eq!(suggestion, "Foo");
        }
        other => panic!("expected UnderscoreInIdentifier, got {:?}", other),
    }
}

#[test]
fn middle_underscore_is_error() {
    let err = lex("bar_baz").unwrap_err();
    match err.kind {
        ErrorKind::UnderscoreInIdentifier { name, suggestion } => {
            assert_eq!(name, "bar_baz");
            assert_eq!(suggestion, "barBaz");
        }
        other => panic!("expected UnderscoreInIdentifier, got {:?}", other),
    }
}

#[test]
fn trailing_underscore_is_error() {
    // `foo_` is also rejected — keeps the rule "no underscores in identifiers"
    // simple to state.
    let err = lex("foo_ bar").unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::UnderscoreInIdentifier { .. }
    ));
}
