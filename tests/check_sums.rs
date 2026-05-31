use i_lang::check::check_file;
use i_lang::check::types::{EffectRow, PrimTy, Ty};
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn sum_type_with_variants_is_registered() {
    let src = "\
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).expect("expected check to succeed");
    let circle = res.defs.iter().find(|d| d.name == "Circle").unwrap();
    let rect = res.defs.iter().find(|d| d.name == "Rect").unwrap();
    assert!(typing.schemes.contains_key(&circle.id));
    assert!(typing.schemes.contains_key(&rect.id));
}

#[test]
fn bare_variant_has_parent_type() {
    let src = "\
type Maybe a
    None
    Some : a

empty : Maybe Int
empty = None
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let e = res.defs.iter().find(|d| d.name == "empty").unwrap();
    let maybe = res.defs.iter().find(|d| d.name == "Maybe").unwrap();
    assert_eq!(
        typing.schemes[&e.id].ty,
        Ty::Con(maybe.id, vec![Ty::Prim(PrimTy::Int)])
    );
}

#[test]
fn single_payload_ctor_takes_payload_type() {
    let src = "\
type Maybe a
    None
    Some : a

three : Maybe Int
three = Some 3
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let t = res.defs.iter().find(|d| d.name == "three").unwrap();
    let maybe = res.defs.iter().find(|d| d.name == "Maybe").unwrap();
    assert_eq!(
        typing.schemes[&t.id].ty,
        Ty::Con(maybe.id, vec![Ty::Prim(PrimTy::Int)])
    );
}

#[test]
fn match_unwraps_maybe() {
    let src = "\
type Maybe a
    None
    Some : a

unwrapOr : Maybe Int, Int -> Int
unwrapOr = m d -> m match
    Some n -> n
    None -> d
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    assert!(
        typing
            .schemes
            .values()
            .any(|s| matches!(&s.ty, Ty::Fun(..)))
    );
}

#[test]
fn match_with_wildcard_on_int_type_checks() {
    let src = "\
classify : Int -> Int
classify = n -> n match
    0 -> 0
    _ -> 1
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let c = res.defs.iter().find(|d| d.name == "classify").unwrap();
    let expected = Ty::Fun(
        vec![Ty::Prim(PrimTy::Int)],
        EffectRow::pure(),
        Box::new(Ty::Prim(PrimTy::Int)),
    );
    assert_eq!(typing.schemes[&c.id].ty, expected);
}

#[test]
fn match_infers_function_type_from_arms_without_annotation() {
    let src = "\
type Maybe a
    None
    Some : a

intOf = m -> m match
    Some n -> n
    None -> 0
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let f = res.defs.iter().find(|d| d.name == "intOf").unwrap();
    let maybe = res.defs.iter().find(|d| d.name == "Maybe").unwrap();
    let expected = Ty::Fun(
        vec![Ty::Con(maybe.id, vec![Ty::Prim(PrimTy::Int)])],
        EffectRow::pure(),
        Box::new(Ty::Prim(PrimTy::Int)),
    );
    assert_eq!(typing.schemes[&f.id].ty, expected);
}

#[test]
fn match_arms_with_mismatched_body_types_errors() {
    // The annotation pins the return type to Int; the second arm's "hi"
    // (String) then fails to unify with Int. Without an annotation,
    // inference would happily reconcile both arms to String and report
    // no error.
    let src = "\
type Maybe a
    None
    Some : a

bad : Maybe Int -> Int
bad = m -> m match
    Some n -> n
    None -> \"hi\"
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(&e.kind, i_lang::error::ErrorKind::TypeMismatch { .. }))
    );
}

#[test]
fn non_exhaustive_match_errors() {
    let src = "\
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

f : Shape -> Float
f = s -> s match
    Circle r -> r
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        i_lang::error::ErrorKind::NonExhaustiveMatch { missing }
            if missing.iter().any(|m| m == "Rect")
    )));
}

#[test]
fn exhaustive_match_with_wildcard_is_ok() {
    let src = "\
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

f : Shape -> Float
f = s -> s match
    Circle r -> r
    _ -> 0.0
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    check_file(&file, &res).unwrap();
}

#[test]
fn ctor_payload_type_mismatch_errors() {
    let src = "\
type Maybe a
    None
    Some : a

bad : Maybe Int
bad = Some \"hi\"
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. }))
    );
}
