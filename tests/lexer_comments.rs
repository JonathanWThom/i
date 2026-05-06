use i_lang::lex::lex;
use i_lang::token::TokenKind::*;

fn kinds(src: &str) -> Vec<i_lang::token::TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn line_comment_consumed() {
    assert_eq!(kinds("# this is a comment"), vec![Eof]);
}

#[test]
fn trailing_comment() {
    // Newline still comes through as a literal byte; layout (Task 9-10)
    // turns it into a Newline token.
    assert_eq!(
        kinds("x = 1 # comment"),
        vec![LowerIdent("x".into()), Equals, IntLit(1), Eof]
    );
}

#[test]
fn comment_does_not_eat_next_line() {
    let src = "# c1\nx";
    assert_eq!(kinds(src), vec![LowerIdent("x".into()), Eof]);
}

#[test]
fn comment_then_token() {
    let src = "# c1\n# c2\ny";
    assert_eq!(kinds(src), vec![LowerIdent("y".into()), Eof]);
}
