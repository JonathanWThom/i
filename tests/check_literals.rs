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

#[test]
fn applied_identity_has_arg_type() {
    let src = "\
id = x -> x
n = id 42
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let n = res.defs.iter().find(|d| d.name == "n").unwrap();
    assert_eq!(typing.schemes[&n.id].ty, Ty::Prim(PrimTy::Int));
}

#[test]
fn arity_mismatch_in_call_reports_error() {
    // `id` takes one arg; pass two.
    let src = "\
id = x -> x
n = id 1, 2
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e.kind, i_lang::error::ErrorKind::ArityMismatch { .. }))
    );
}

#[test]
fn identity_lambda_has_fun_type() {
    let src = "id = x -> x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let id = res.defs.iter().find(|d| d.name == "id").unwrap();
    match &typing.schemes[&id.id].ty {
        Ty::Fun(params, result) => {
            assert_eq!(params.len(), 1);
            assert_eq!(&params[0], result.as_ref());
        }
        other => panic!("expected Fun, got {:?}", other),
    }
}

#[test]
fn int_addition_is_int() {
    let src = "n = 1 + 2\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let n = res.defs.iter().find(|d| d.name == "n").unwrap();
    assert_eq!(typing.schemes[&n.id].ty, Ty::Prim(PrimTy::Int));
}

#[test]
fn mixed_int_float_addition_errors() {
    let src = "n = 1 + 1.0\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. }))
    );
}

#[test]
fn comparison_returns_bool() {
    let src = "b = 1 == 2\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let b = res.defs.iter().find(|d| d.name == "b").unwrap();
    assert_eq!(typing.schemes[&b.id].ty, Ty::Prim(PrimTy::Bool));
}

#[test]
fn logical_and_requires_bool() {
    let src = "b = 1 and 2\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. }))
    );
}

#[test]
fn empty_list_is_list_of_var() {
    let src = "\
type List a
    Empty
    Cons
        head : a
        tail : List a

xs : List Int
xs = []
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let xs = res.defs.iter().find(|d| d.name == "xs").unwrap();
    let list_def = res.defs.iter().find(|d| d.name == "List").unwrap();
    assert_eq!(
        typing.schemes[&xs.id].ty,
        Ty::Con(list_def.id, vec![Ty::Prim(PrimTy::Int)])
    );
}

#[test]
fn homogeneous_list_takes_element_type() {
    let src = "\
type List a
    Empty
    Cons
        head : a
        tail : List a

xs = [1, 2, 3]
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let xs = res.defs.iter().find(|d| d.name == "xs").unwrap();
    let list_def = res.defs.iter().find(|d| d.name == "List").unwrap();
    assert_eq!(
        typing.schemes[&xs.id].ty,
        Ty::Con(list_def.id, vec![Ty::Prim(PrimTy::Int)])
    );
}

#[test]
fn heterogeneous_list_errors() {
    let src = "\
type List a
    Empty
    Cons
        head : a
        tail : List a

xs = [1, \"hi\"]
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. }))
    );
}
