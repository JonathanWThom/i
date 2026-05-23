use i_lang::error::ErrorKind;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::{ResolvedName, resolve_file};

#[test]
fn var_resolves_to_top_level() {
    let src = "module M\n    expose y\n\nx = 1\ny = x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let x_resolutions: Vec<_> = res
        .refs
        .values()
        .filter(|r| matches!(r, ResolvedName::TopLevel(_)))
        .collect();
    assert_eq!(x_resolutions.len(), 1);
}

#[test]
fn unknown_var_is_error() {
    let src = "module M\n    expose y\n\ny = unknownName\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "unknownName"
    )));
}
