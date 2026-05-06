use i_lang::lex::lex;
use i_lang::token::TokenKind::*;

fn kinds(src: &str) -> Vec<i_lang::token::TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn integers() {
    assert_eq!(
        kinds("0 42 1000"),
        vec![IntLit(0), IntLit(42), IntLit(1000), Eof]
    );
}

#[test]
fn floats() {
    assert_eq!(
        kinds("2.5 0.0 100.5"),
        vec![FloatLit(2.5), FloatLit(0.0), FloatLit(100.5), Eof]
    );
}

#[test]
fn negative_is_two_tokens() {
    // per Plan 2 decision: unary minus + literal
    assert_eq!(
        kinds("-3 -2.5"),
        vec![Minus, IntLit(3), Minus, FloatLit(2.5), Eof]
    );
}

#[test]
fn dot_after_digits_is_float() {
    assert_eq!(kinds("2.5"), vec![FloatLit(2.5), Eof]);
}

#[test]
fn dot_method_call_not_float() {
    // digits.dot.alpha is int + dot + ident; the dot is a method/field access
    assert_eq!(
        kinds("3.foo"),
        vec![IntLit(3), Dot, LowerIdent("foo".into()), Eof]
    );
}

#[test]
fn dotdot_after_int_is_int_then_dotdot() {
    // `3..` shouldn't try to read `.` as a float dot.
    assert_eq!(kinds("3.."), vec![IntLit(3), DotDot, Eof]);
}
