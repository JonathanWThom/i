use i_lang::lex::lex;
use i_lang::token::TokenKind::*;

fn kinds(src: &str) -> Vec<i_lang::token::TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn newline_terminates_line() {
    assert_eq!(
        kinds("x = 1\ny = 2\n"),
        vec![
            LowerIdent("x".into()),
            Equals,
            IntLit(1),
            Newline,
            LowerIdent("y".into()),
            Equals,
            IntLit(2),
            Newline,
            Eof,
        ]
    );
}

#[test]
fn blank_lines_collapsed() {
    // Multiple consecutive newlines collapse into one.
    assert_eq!(
        kinds("x\n\n\ny\n"),
        vec![
            LowerIdent("x".into()),
            Newline,
            LowerIdent("y".into()),
            Newline,
            Eof,
        ]
    );
}

#[test]
fn leading_blank_lines_ignored() {
    // Newlines before the first token don't produce a token.
    assert_eq!(kinds("\n\nx\n"), vec![LowerIdent("x".into()), Newline, Eof]);
}

#[test]
fn trailing_arith_op_continues_line() {
    assert_eq!(
        kinds("a +\nb"),
        vec![
            LowerIdent("a".into()),
            Plus,
            LowerIdent("b".into()),
            Eof,
        ]
    );
}

#[test]
fn trailing_comparison_continues_line() {
    assert_eq!(
        kinds("a <=\nb"),
        vec![
            LowerIdent("a".into()),
            LtEq,
            LowerIdent("b".into()),
            Eof,
        ]
    );
}

#[test]
fn open_paren_continues_line() {
    // Inside parens, no Newline tokens emitted.
    assert_eq!(
        kinds("f (\n  x\n)"),
        vec![
            LowerIdent("f".into()),
            LParen,
            LowerIdent("x".into()),
            RParen,
            Eof,
        ]
    );
}

#[test]
fn open_bracket_continues_line() {
    assert_eq!(
        kinds("[\n  1,\n  2\n]"),
        vec![LBracket, IntLit(1), Comma, IntLit(2), RBracket, Eof]
    );
}

#[test]
fn comma_continues_line() {
    assert_eq!(
        kinds("f a,\n  b"),
        vec![
            LowerIdent("f".into()),
            LowerIdent("a".into()),
            Comma,
            LowerIdent("b".into()),
            Eof,
        ]
    );
}

#[test]
fn equals_does_not_continue_line() {
    // `=` followed by newline starts a block body; newline must be emitted.
    let toks = kinds("main =\nx");
    assert_eq!(
        toks,
        vec![
            LowerIdent("main".into()),
            Equals,
            Newline,
            LowerIdent("x".into()),
            Eof,
        ]
    );
}

#[test]
fn arrow_does_not_continue_line() {
    // `->` followed by newline starts a block body; newline must be emitted.
    let toks = kinds("f = x ->\ny");
    assert_eq!(
        toks,
        vec![
            LowerIdent("f".into()),
            Equals,
            LowerIdent("x".into()),
            Arrow,
            Newline,
            LowerIdent("y".into()),
            Eof,
        ]
    );
}
