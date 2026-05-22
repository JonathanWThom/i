use i_lang::lex::lex;
use i_lang::parse::parse_expr_for_test;

fn p(src: &str) -> String {
    format!("{}", parse_expr_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn add() {
    assert_eq!(p("1 + 2"), "(+ (int 1) (int 2))");
}

#[test]
fn add_left_assoc() {
    assert_eq!(p("1 + 2 + 3"), "(+ (+ (int 1) (int 2)) (int 3))");
}

#[test]
fn mul_higher_than_add() {
    assert_eq!(p("1 + 2 * 3"), "(+ (int 1) (* (int 2) (int 3)))");
}

#[test]
fn pow_right_assoc() {
    assert_eq!(p("2 ^ 3 ^ 2"), "(^ (int 2) (^ (int 3) (int 2)))");
}

#[test]
fn unary_minus() {
    assert_eq!(p("-3"), "(neg (int 3))");
}

#[test]
fn compare_non_assoc() {
    let toks = lex("a < b < c").unwrap();
    let err = parse_expr_for_test(&toks).unwrap_err();
    assert!(matches!(
        err.kind,
        i_lang::error::ErrorKind::ChainedComparison
    ));
}

#[test]
fn concat_right() {
    assert_eq!(
        p(r#""a" ++ "b" ++ "c""#),
        r#"(++ (str "a") (++ (str "b") (str "c")))"#
    );
}

#[test]
fn lambda_simple() {
    assert_eq!(p("x -> x + 1"), "(lambda ((pvar x)) (+ (var x) (int 1)))");
}

#[test]
fn lambda_multi_param() {
    assert_eq!(
        p("a b -> a + b"),
        "(lambda ((pvar a) (pvar b)) (+ (var a) (var b)))"
    );
}

#[test]
fn lambda_body_greedy() {
    assert_eq!(
        p("x -> x + 1 + 2"),
        "(lambda ((pvar x)) (+ (+ (var x) (int 1)) (int 2)))"
    );
}

#[test]
fn or_left_assoc() {
    assert_eq!(p("a or b or c"), "(or (or (var a) (var b)) (var c))");
}

#[test]
fn and_higher_than_or() {
    assert_eq!(p("a or b and c"), "(or (var a) (and (var b) (var c)))");
}

#[test]
fn not_prefix() {
    assert_eq!(p("not x"), "(not (var x))");
}
