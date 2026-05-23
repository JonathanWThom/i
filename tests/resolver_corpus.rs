use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn snapshot_resolver_corpus() {
    insta::glob!(
        env!("CARGO_MANIFEST_DIR"),
        "tests/corpus/resolver/*.i",
        |path| {
            let src = std::fs::read_to_string(path).unwrap();
            let toks = lex(&src).unwrap();
            let file = parse(&toks).unwrap();
            let res = resolve_file(&file).expect("corpus fixtures must resolve");
            insta::assert_snapshot!(format!("{}", res));
        }
    );
}
