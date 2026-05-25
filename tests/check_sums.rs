use i_lang::check::check_file;
use i_lang::check::types::{PrimTy, Ty};
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
