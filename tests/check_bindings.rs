use i_lang::check::check_file;
use i_lang::check::types::{PrimTy, Ty};
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn mutual_top_level_recursion_typechecks() {
    let src = "\
a = b
b = 1
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).expect("expected mutual-rec to type-check");
    let a = res.defs.iter().find(|d| d.name == "a").unwrap();
    let b = res.defs.iter().find(|d| d.name == "b").unwrap();
    assert_eq!(typing.schemes[&a.id].ty, Ty::Prim(PrimTy::Int));
    assert_eq!(typing.schemes[&b.id].ty, Ty::Prim(PrimTy::Int));
}
