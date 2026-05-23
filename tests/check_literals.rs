use i_lang::check::check_file;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn empty_top_level_binding_returns_empty_typing() {
    let src = "x = 1\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).expect("expected check to succeed");
    assert!(typing.expr_types.is_empty());
    assert!(typing.schemes.is_empty());
}
