use i_lang::error::ErrorKind;
use i_lang::lex::lex;
use i_lang::token::TokenKind::*;

fn kinds(src: &str) -> Vec<i_lang::token::TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn plain_string() {
    assert_eq!(kinds(r#""hello""#), vec![StringLit("hello".into()), Eof]);
}

#[test]
fn escaped_chars() {
    assert_eq!(
        kinds(r#""a\nb\tc\\d\"e\rf\0g""#),
        vec![StringLit("a\nb\tc\\d\"e\rf\0g".into()), Eof]
    );
}

#[test]
fn empty_string() {
    assert_eq!(kinds(r#""""#), vec![StringLit(String::new()), Eof]);
}

#[test]
fn two_strings() {
    assert_eq!(
        kinds(r#""hi" "there""#),
        vec![StringLit("hi".into()), StringLit("there".into()), Eof,]
    );
}

#[test]
fn unterminated_at_eof() {
    let err = lex(r#""unterminated"#).unwrap_err();
    assert!(matches!(err.kind, ErrorKind::UnterminatedString));
}

#[test]
fn unterminated_at_newline() {
    let err = lex("\"oops\n").unwrap_err();
    assert!(matches!(err.kind, ErrorKind::UnterminatedString));
}

#[test]
fn bad_escape() {
    let err = lex(r#""bad \q escape""#).unwrap_err();
    match err.kind {
        ErrorKind::InvalidEscape(c) => assert_eq!(c, 'q'),
        other => panic!("expected InvalidEscape, got {:?}", other),
    }
}
