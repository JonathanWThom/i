use i_lang::ast::File;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::pretty::pretty;

fn parse_str(src: &str) -> File {
    parse(&lex(src).unwrap()).unwrap()
}

fn roundtrip(src: &str) {
    let ast1 = parse_str(src);
    let printed = pretty(&ast1);
    let ast2 = parse_str(&printed);
    assert!(
        ast1.node_eq(&ast2),
        "round-trip differs for source:\n{}\nprinted:\n{}",
        src,
        printed
    );
}

#[test]
fn examples_roundtrip() {
    for entry in std::fs::read_dir("examples").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("i") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        eprintln!("round-tripping {}", path.display());
        roundtrip(&src);
    }
}

#[test]
fn corpus_roundtrip() {
    for entry in std::fs::read_dir("tests/corpus/parser").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("i") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        eprintln!("round-tripping {}", path.display());
        roundtrip(&src);
    }
}
