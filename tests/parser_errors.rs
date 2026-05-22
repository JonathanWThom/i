use i_lang::error::ErrorKind;
use i_lang::lex::lex;
use i_lang::parse::parse;

fn parse_err(src: &str) -> ErrorKind {
    parse(&lex(src).unwrap()).unwrap_err().kind
}

#[test]
fn chained_comparison() {
    assert!(matches!(
        parse_err("x = a < b < c\n"),
        ErrorKind::ChainedComparison
    ));
}

#[test]
fn match_without_indent() {
    let err = parse_err("x = n match\n");
    assert!(matches!(err, ErrorKind::Unexpected { .. }));
}

#[test]
fn missing_paren() {
    let err = parse_err("x = (1 + 2\n");
    assert!(matches!(err, ErrorKind::Unexpected { .. }));
}

#[test]
fn empty_match() {
    let src = "x = n match\n    \n";
    let err = parse_err(src);
    assert!(matches!(err, ErrorKind::Unexpected { .. }));
}
