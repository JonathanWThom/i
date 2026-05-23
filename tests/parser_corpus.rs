use i_lang::lex::lex;
use i_lang::parse::parse;

#[test]
fn snapshot_examples() {
    insta::glob!(env!("CARGO_MANIFEST_DIR"), "examples/*.i", |path| {
        let src = std::fs::read_to_string(path).unwrap();
        let toks = lex(&src).unwrap();
        let file = parse(&toks).unwrap();
        insta::assert_snapshot!(format!("{}", file));
    });
}

#[test]
fn snapshot_corpus() {
    insta::glob!(
        env!("CARGO_MANIFEST_DIR"),
        "tests/corpus/parser/*.i",
        |path| {
            let src = std::fs::read_to_string(path).unwrap();
            let toks = lex(&src).unwrap();
            let file = parse(&toks).unwrap();
            insta::assert_snapshot!(format!("{}", file));
        }
    );
}
