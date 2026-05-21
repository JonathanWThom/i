use i_lang::lex::lex;
use i_lang::parse::parse_expr_for_test;

fn p(src: &str) -> String {
    format!("{}", parse_expr_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn construction() {
    assert_eq!(
        p("Point(x = 0, y = 0)"),
        "(construct Point (kw x (int 0)) (kw y (int 0)))"
    );
}

#[test]
fn update() {
    assert_eq!(p("p1(x = 5)"), "(update (var p1) (kw x (int 5)))");
}

#[test]
fn nested_construction() {
    assert_eq!(
        p("Pair(left = Point(x = 0, y = 0), right = None)"),
        "(construct Pair (kw left (construct Point (kw x (int 0)) (kw y (int 0)))) (kw right (ctor None)))"
    );
}
