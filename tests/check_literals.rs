use i_lang::check::check_file;
use i_lang::check::types::{PrimTy, Ty};
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

fn check_value(src: &str, name: &str) -> Ty {
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let def = res.defs.iter().find(|d| d.name == name).unwrap();
    typing.schemes[&def.id].ty.clone()
}

#[test]
fn int_literal_has_type_int() {
    assert_eq!(check_value("x = 1\n", "x"), Ty::Prim(PrimTy::Int));
}

#[test]
fn float_literal_has_type_float() {
    assert_eq!(check_value("x = 1.5\n", "x"), Ty::Prim(PrimTy::Float));
}

#[test]
fn string_literal_has_type_string() {
    assert_eq!(check_value("x = \"hi\"\n", "x"), Ty::Prim(PrimTy::String));
}
