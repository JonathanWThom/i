use i_lang::lex::lex;
use i_lang::parse::parse_expr_for_test;

fn p(src: &str) -> String {
    format!("{}", parse_expr_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn match_simple() {
    let src = "n match\n    0 -> \"zero\"\n    _ -> \"other\"";
    assert_eq!(
        p(src),
        "(match (var n) ((arm (plit (int 0)) (str \"zero\")) (arm (wild) (str \"other\"))))"
    );
}

#[test]
fn match_constructor() {
    let src = "shape match\n    Circle r -> r\n    Rect w, h -> w";
    assert_eq!(
        p(src),
        "(match (var shape) ((arm (pctor Circle (pvar r)) (var r)) (arm (pctor Rect (pvar w) (pvar h)) (var w))))"
    );
}
