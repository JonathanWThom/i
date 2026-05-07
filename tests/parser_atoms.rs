use i_lang::lex::lex;
use i_lang::parse::parse_expr_for_test;

fn parse(src: &str) -> String {
    let toks = lex(src).unwrap();
    let e = parse_expr_for_test(&toks).unwrap();
    format!("{}", e.node)
}

#[test]
fn int_literal() {
    assert_eq!(parse("42"), "(int 42)");
}

#[test]
fn float_literal() {
    assert_eq!(parse("3.14"), "(float 3.14)");
}

#[test]
fn string_literal() {
    assert_eq!(parse(r#""hi""#), r#"(str "hi")"#);
}

#[test]
fn lower_var() {
    assert_eq!(parse("foo"), "(var foo)");
}

#[test]
fn upper_ctor() {
    assert_eq!(parse("None"), "(ctor None)");
}

#[test]
fn paren_group() {
    assert_eq!(parse("(42)"), "(paren (int 42))");
}

#[test]
fn list_literal() {
    assert_eq!(parse("[1, 2, 3]"), "(list (int 1) (int 2) (int 3))");
}

#[test]
fn empty_list() {
    assert_eq!(parse("[]"), "(list)");
}
