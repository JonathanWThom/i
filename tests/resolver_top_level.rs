use i_lang::error::ErrorKind;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn empty_file_resolves() {
    let src = "module M\n    expose x\n\nx = 1\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    assert_eq!(res.defs.len(), 1);
    assert_eq!(res.defs[0].name, "x");
}

#[test]
fn collects_multiple_top_level() {
    let src = "module M\n    expose x, y\n\nx = 1\ny = 2\n\ntype Int\n    v : Int\n\ntype Pair\n    a : Int\n    b : Int\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let names: Vec<&str> = res.defs.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
    assert!(names.contains(&"Pair"));
}

#[test]
fn duplicate_value_binding() {
    let src = "module M\n    expose x\n\nx = 1\nx = 2\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::DuplicateDefinition { name, .. } if name == "x"
    )));
}
