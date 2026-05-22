use i_lang::lex::lex;
use i_lang::parse::parse;

fn p(src: &str) -> String {
    format!("{}", parse(&lex(src).unwrap()).unwrap())
}

#[test]
fn newtype_short() {
    assert_eq!(
        p("type UserId = Int"),
        "(file\n  (type UserId (tnamed Int)))"
    );
}

#[test]
fn record_two_fields() {
    let src = "type Point\n    x : Float\n    y : Float";
    assert_eq!(
        p(src),
        "(file\n  (type Point\n    (field x (tnamed Float))\n    (field y (tnamed Float))))"
    );
}

#[test]
fn parametric_record() {
    let src = "type Box a\n    value : a";
    assert_eq!(p(src), "(file\n  (type Box a\n    (field value (tvar a))))");
}

#[test]
fn sum_bare_variants() {
    let src = "type Color\n    Red\n    Green\n    Blue";
    assert_eq!(
        p(src),
        "(file\n  (type Color\n    (variant Red)\n    (variant Green)\n    (variant Blue)))"
    );
}

#[test]
fn sum_with_single_payload() {
    let src = "type Maybe a\n    None\n    Some : a";
    assert_eq!(
        p(src),
        "(file\n  (type Maybe a\n    (variant None)\n    (variant Some (tvar a))))"
    );
}

#[test]
fn variant_with_field_block() {
    let src = "type Shape\n    Circle\n        radius : Float";
    assert_eq!(
        p(src),
        "(file\n  (type Shape\n    (variant Circle\n      (field radius (tnamed Float)))))"
    );
}

#[test]
fn trait_decl() {
    let src = "trait Eq a\n    eq : a, a -> Bool";
    assert_eq!(
        p(src),
        "(file\n  (trait Eq a\n    (sig eq (tfun (tvar a) (tvar a) (tnamed Bool)))))"
    );
}

#[test]
fn impl_decl() {
    let src = "impl Eq Point\n    eq = a b -> a";
    assert_eq!(
        p(src),
        "(file\n  (impl Eq (tnamed Point)\n    (let eq (lambda ((pvar a) (pvar b)) (var a)))))"
    );
}
