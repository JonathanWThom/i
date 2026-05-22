use i_lang::lex::lex;
use i_lang::parse::parse_pattern_for_test;

fn p(src: &str) -> String {
    format!(
        "{}",
        parse_pattern_for_test(&lex(src).unwrap()).unwrap().node
    )
}

#[test]
fn wildcard() {
    assert_eq!(p("_"), "(wild)");
}

#[test]
fn var_pattern() {
    assert_eq!(p("x"), "(pvar x)");
}

#[test]
fn int_pattern() {
    assert_eq!(p("42"), "(plit (int 42))");
}

#[test]
fn ctor_no_args() {
    assert_eq!(p("None"), "(pctor None)");
}

#[test]
fn ctor_with_args() {
    assert_eq!(p("Some x"), "(pctor Some (pvar x))");
}

#[test]
fn tuple_pattern() {
    assert_eq!(p("(a, b)"), "(ptuple (pvar a) (pvar b))");
}

#[test]
fn list_pattern() {
    assert_eq!(p("[a, b]"), "(plist (pvar a) (pvar b))");
}

#[test]
fn record_pattern() {
    assert_eq!(
        p("Point(x = a, y = b)"),
        "(precord Point (pf x (pvar a)) (pf y (pvar b)))"
    );
}
