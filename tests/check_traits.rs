use i_lang::check::check_file;
use i_lang::error::ErrorKind;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

fn errors(src: &str) -> Vec<ErrorKind> {
    let toks = lex(src).expect("lex");
    let file = parse(&toks).expect("parse");
    let res = resolve_file(&file).expect("resolve");
    match check_file(&file, &res) {
        Ok(_) => vec![],
        Err(es) => es.into_iter().map(|e| e.kind).collect(),
    }
}

#[test]
fn impl_of_unknown_trait_errors() {
    let src = "type Point\n    x : Int\nimpl Bogus Point\n    eq = a b -> a\n";
    assert!(
        errors(src)
            .iter()
            .any(|k| matches!(k, ErrorKind::UnknownTrait { name } if name == "Bogus"))
    );
}

#[test]
fn duplicate_impl_errors() {
    let src = "type Point\n    x : Int\nimpl Eq Point\n    eq = a b -> a.x == b.x\n    ne = a b -> not (a.x == b.x)\nimpl Eq Point\n    eq = a b -> a.x == b.x\n    ne = a b -> not (a.x == b.x)\n";
    assert!(
        errors(src).iter().any(
            |k| matches!(k, ErrorKind::DuplicateImpl { trait_name, .. } if trait_name == "Eq")
        )
    );
}

#[test]
fn impl_missing_method_errors() {
    // Eq requires both eq and ne; provide only eq.
    let src = "type Point\n    x : Int\nimpl Eq Point\n    eq = a b -> a.x == b.x\n";
    assert!(
        errors(src)
            .iter()
            .any(|k| matches!(k, ErrorKind::MissingMethod { method, .. } if method == "ne"))
    );
}

#[test]
fn impl_unknown_method_errors() {
    let src = "type Point\n    x : Int\nimpl Eq Point\n    eq = a b -> a.x == b.x\n    ne = a b -> not (a.x == b.x)\n    zz = a b -> a\n";
    assert!(
        errors(src)
            .iter()
            .any(|k| matches!(k, ErrorKind::UnknownMethod { method, .. } if method == "zz"))
    );
}
