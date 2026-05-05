use i_lang::lex::lex;
use i_lang::token::TokenKind::*;

fn kinds(src: &str) -> Vec<i_lang::token::TokenKind> {
    lex(src).unwrap().into_iter().map(|t| t.kind).collect()
}

#[test]
fn single_char_punct() {
    assert_eq!(
        kinds("()[],.:= !?"),
        vec![
            LParen, RParen, LBracket, RBracket, Comma, Dot, Colon, Equals, Bang, Question, Eof,
        ]
    );
}

#[test]
fn multi_char_operators() {
    assert_eq!(
        kinds("-> == /= <= >= ++ .."),
        vec![Arrow, EqEq, SlashEq, LtEq, GtEq, PlusPlus, DotDot, Eof]
    );
}

#[test]
fn arithmetic_operators() {
    assert_eq!(
        kinds("+ - * / ^ < >"),
        vec![Plus, Minus, Star, Slash, Caret, Lt, Gt, Eof]
    );
}
