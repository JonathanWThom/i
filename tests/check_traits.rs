use i_lang::check::check_file;
use i_lang::check::traits::TraitId;
use i_lang::check::types::{PrimTy, Ty};
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

fn check_ok(src: &str) -> i_lang::check::Typing {
    let toks = lex(src).expect("lex");
    let file = parse(&toks).expect("parse");
    let res = resolve_file(&file).expect("resolve");
    check_file(&file, &res).expect("check")
}

#[test]
fn int_addition_still_types_as_int() {
    // Operator now dispatches via Add, but 3 + 4 must still type as Int
    // (Add Int is a seeded built-in impl).
    let t = check_ok("main = 3 + 4\n");
    let scheme = t.schemes.values().next().expect("a scheme");
    assert_eq!(scheme.ty, Ty::Prim(PrimTy::Int));
}

#[test]
fn comparison_still_types_as_bool() {
    let t = check_ok("main = 3 < 4\n");
    let scheme = t.schemes.values().next().unwrap();
    assert_eq!(scheme.ty, Ty::Prim(PrimTy::Bool));
}

#[test]
fn operator_on_type_without_impl_errors() {
    // Point has no Eq impl; `pt == pt` must fail with MissingImpl.
    let src = "type Point\n    x : Int\np : Point\np = Point(x = 1)\nb = p == p\n";
    assert!(
        errors(src)
            .iter()
            .any(|k| matches!(k, ErrorKind::MissingImpl { trait_name, .. } if trait_name == "Eq"))
    );
}

#[test]
fn operator_on_type_with_impl_ok() {
    let src = "type Point\n    x : Int\nimpl Eq Point\n    eq = a b -> a.x == b.x\n    ne = a b -> not (a.x == b.x)\np : Point\np = Point(x = 1)\nb = p == p\n";
    let errs = errors(src);
    assert!(errs.is_empty(), "expected clean, got {errs:?}");
}

#[test]
fn generic_equality_helper_infers_eq_constraint() {
    // bothEq compares its args; its scheme should carry Eq on the param var.
    let src = "bothEq = a b -> a == b\n";
    let t = check_ok(src);
    let scheme = t
        .schemes
        .values()
        .find(|s| matches!(s.ty, Ty::Fun(..)))
        .unwrap();
    assert_eq!(scheme.constraints.len(), 1, "scheme: {scheme:?}");
    assert_eq!(scheme.constraints[0].trait_, TraitId::Eq);
    if let Ty::Var(v) = scheme.constraints[0].ty {
        assert!(scheme.vars.contains(&v));
    } else {
        panic!(
            "expected constraint on a type var, got {:?}",
            scheme.constraints[0].ty
        );
    }
}

#[test]
fn unsatisfiable_monomorphic_constraint_is_ambiguous() {
    // A block-local lambda is monomorphic; its Eq'd param var never resolves
    // and never generalises, so the constraint is ambiguous.
    let src = "main =\n    f = x -> x == x\n    0\n";
    assert!(
        errors(src)
            .iter()
            .any(|k| matches!(k, ErrorKind::AmbiguousConstraint { .. }))
    );
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
