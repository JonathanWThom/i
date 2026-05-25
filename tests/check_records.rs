use i_lang::check::check_file;
use i_lang::check::types::Ty;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn newtype_declaration_is_nominal() {
    // A newtype is a distinct nominal type. `firstUser : UserId` and
    // `firstUser = 1` should TypeMismatch — `1 : Int` doesn't unify with UserId
    // even though UserId is "the same as" Int structurally.
    let src = "\
type UserId = Int
firstUser : UserId
firstUser = 1
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. }))
    );
}

#[test]
fn record_construction_with_all_fields_succeeds() {
    let src = "\
type Point
    x : Float
    y : Float

origin = Point(x = 0.0, y = 0.0)
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let origin = res.defs.iter().find(|d| d.name == "origin").unwrap();
    let point = res.defs.iter().find(|d| d.name == "Point").unwrap();
    assert_eq!(typing.schemes[&origin.id].ty, Ty::Con(point.id, vec![]));
}

#[test]
fn record_construction_missing_field_errors() {
    let src = "\
type Point
    x : Float
    y : Float

bad = Point(x = 0.0)
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e.kind, i_lang::error::ErrorKind::MissingField { .. }))
    );
}

#[test]
fn record_construction_unknown_field_errors() {
    let src = "\
type Point
    x : Float
    y : Float

bad = Point(x = 0.0, y = 0.0, z = 0.0)
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e.kind, i_lang::error::ErrorKind::UnknownField { .. }))
    );
}

#[test]
fn record_update_keeps_type() {
    let src = "\
type Point
    x : Float
    y : Float

p1 = Point(x = 0.0, y = 0.0)
p2 = p1(x = 5.0)
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let p1 = res.defs.iter().find(|d| d.name == "p1").unwrap();
    let p2 = res.defs.iter().find(|d| d.name == "p2").unwrap();
    assert_eq!(typing.schemes[&p1.id].ty, typing.schemes[&p2.id].ty);
}

#[test]
fn newtype_block_with_construct_passes() {
    // Deferred from Task 12: block-form newtype + record construction.
    let src = "\
type UserId
    value : Int

firstUser : UserId
firstUser = UserId(value = 1)
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).unwrap();
    let f = res.defs.iter().find(|d| d.name == "firstUser").unwrap();
    let user_id = res.defs.iter().find(|d| d.name == "UserId").unwrap();
    assert_eq!(typing.schemes[&f.id].ty, Ty::Con(user_id.id, vec![]));
}
