use i_lang::lex::lex;

#[test]
fn empty_program() {
    let toks = lex("").unwrap();
    assert_eq!(toks.len(), 1);
    assert!(matches!(toks[0].kind, i_lang::token::TokenKind::Eof));
}
