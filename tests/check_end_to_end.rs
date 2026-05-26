use i_lang::check::check_file;
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
        Ty::Fun(params, ret) => {
            assert_eq!(params.len(), 1);
            assert_eq!(ret.as_ref(), &Ty::Prim(PrimTy::Float));
        }
        other => panic!("expected Fun, got {:?}", other),
    }
}
