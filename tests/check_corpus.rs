use i_lang::check::check_file;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn snapshot_check_corpus() {
    insta::glob!(
        env!("CARGO_MANIFEST_DIR"),
        "tests/corpus/check/*.i",
        |path| {
            let src = std::fs::read_to_string(path).unwrap();
            let toks = lex(&src).expect("lex");
            let file = parse(&toks).expect("parse");
            let res = resolve_file(&file).expect("resolve");
            let typing = check_file(&file, &res).expect("check");
            insta::assert_snapshot!(format!("{}", typing));
        }
    );
}
