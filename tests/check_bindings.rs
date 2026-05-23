use i_lang::check::check_file;
use i_lang::check::types::{PrimTy, Ty};
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn alias_binding_takes_referent_type() {
    let src = "\
n = 42
m = n
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let m = res.defs.iter().find(|d| d.name == "m").unwrap();
    assert_eq!(typing.schemes[&m.id].ty, Ty::Prim(PrimTy::Int));
}

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

#[test]
fn block_local_takes_inferred_type() {
    let src = "\
f =
    n = 42
    n
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let f = res.defs.iter().find(|d| d.name == "f").unwrap();
    assert_eq!(typing.schemes[&f.id].ty, Ty::Prim(PrimTy::Int));
}

#[test]
fn identity_is_generalised_to_forall() {
    let src = "id = x -> x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let id = res.defs.iter().find(|d| d.name == "id").unwrap();
    let scheme = &typing.schemes[&id.id];
    assert_eq!(
        scheme.vars.len(),
        1,
        "expected one quantified var, got {:?}",
        scheme
    );
    match &scheme.ty {
        Ty::Fun(params, result) => {
            assert_eq!(params.len(), 1);
            assert_eq!(&params[0], result.as_ref());
            assert!(matches!(params[0], Ty::Var(_)));
        }
        other => panic!("expected Fun, got {:?}", other),
    }
}

#[test]
fn polymorphic_top_level_instantiates_at_each_use() {
    let src = "\
id = x -> x
n = id 1
s = id \"hi\"
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let n = res.defs.iter().find(|d| d.name == "n").unwrap();
    let s = res.defs.iter().find(|d| d.name == "s").unwrap();
    assert_eq!(typing.schemes[&n.id].ty, Ty::Prim(PrimTy::Int));
    assert_eq!(typing.schemes[&s.id].ty, Ty::Prim(PrimTy::String));
}

#[test]
fn annotation_pins_inferred_type() {
    let src = "\
double : Int -> Int
double = n -> n
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let d = res.defs.iter().find(|d| d.name == "double").unwrap();
    let expected = Ty::Fun(vec![Ty::Prim(PrimTy::Int)], Box::new(Ty::Prim(PrimTy::Int)));
    assert_eq!(typing.schemes[&d.id].ty, expected);
}

#[test]
fn block_local_is_monomorphic() {
    // id bound inside a block is monomorphic — its tyvar fixes after the first
    // call, so a second call with a different arg type errors.
    let src = "\
result =
    id = x -> x
    n = id 1
    s = id \"hi\"
    n
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. }))
    );
}
