use i_lang::check::check_file;
use i_lang::check::traits::TraitId;
use i_lang::check::types::{PrimTy, Ty};
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn small_program_with_records_methods_and_match_type_checks() {
    let src = "\
type Maybe a
    None
    Some : a

type Point
    x : Float
    y : Float
    magnitude = (self.x * self.x + self.y * self.y) ^ 0.5

origin : Point
origin = Point(x = 0.0, y = 0.0)

magOrZero : Maybe Point -> Float
magOrZero = mp -> mp match
    Some p -> p.magnitude
    None -> 0.0
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).expect("expected end-to-end check to pass");
    let mag = res.defs.iter().find(|d| d.name == "magOrZero").unwrap();
    let result = &typing.schemes[&mag.id].ty;
    match result {
        Ty::Fun(params, _row, ret) => {
            assert_eq!(params.len(), 1);
            assert_eq!(ret.as_ref(), &Ty::Prim(PrimTy::Float));
        }
        other => panic!("expected Fun, got {:?}", other),
    }
}

#[test]
fn program_with_impl_and_generic_helper_type_checks() {
    let src = "\
type Box
    v : Int
impl Eq Box
    eq = a b -> a.v == b.v
    ne = a b -> not (a.v == b.v)
allEqual = a b -> a == b
first : Box
first = Box(v = 1)
second : Box
second = Box(v = 2)
areEqual = allEqual first, second
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).expect("expected end-to-end check to pass");

    // allEqual generalises with an Eq constraint on its shared param var.
    let all_equal = res.defs.iter().find(|d| d.name == "allEqual").unwrap();
    let scheme = &typing.schemes[&all_equal.id];
    assert_eq!(scheme.constraints.len(), 1, "scheme: {scheme:?}");
    assert_eq!(scheme.constraints[0].trait_, TraitId::Eq);

    // Applying it to two Boxes discharges Eq Box: result is Bool, and the
    // obligation is gone (not leaked onto areEqual's scheme).
    let are_equal = res.defs.iter().find(|d| d.name == "areEqual").unwrap();
    assert_eq!(typing.schemes[&are_equal.id].ty, Ty::Prim(PrimTy::Bool));
    assert!(typing.schemes[&are_equal.id].constraints.is_empty());
}
