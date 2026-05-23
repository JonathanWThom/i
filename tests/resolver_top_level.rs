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
