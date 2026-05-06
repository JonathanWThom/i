use i_lang::lex::lex;

#[test]
fn empty_program() {
    let toks = lex("").unwrap();
    assert_eq!(toks.len(), 1);
    assert!(matches!(toks[0].kind, i_lang::token::TokenKind::Eof));
}

#[test]
fn snapshot_examples() {
    insta::glob!(env!("CARGO_MANIFEST_DIR"), "examples/*.i", |path| {
        let src = std::fs::read_to_string(path).unwrap();
        let toks = lex(&src).expect("examples must lex");
        let formatted: String = toks.iter().map(|t| format!("{}\n", t)).collect();
        insta::assert_snapshot!(formatted);
    });
}
