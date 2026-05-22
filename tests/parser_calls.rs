use i_lang::lex::lex;
use i_lang::parse::parse_expr_for_test;

fn p(src: &str) -> String {
    format!("{}", parse_expr_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn paren_free_call() {
    assert_eq!(p("add 3, 4"), "(call (var add) (int 3) (int 4))");
}

#[test]
fn nested_call_with_parens() {
    assert_eq!(
        p("add 3, (mul 4, 5)"),
        "(call (var add) (int 3) (paren (call (var mul) (int 4) (int 5))))"
    );
}

#[test]
fn field_access() {
    assert_eq!(p("p.x"), "(. (var p) x)");
}

#[test]
fn method_chain_atom_only() {
    assert_eq!(
        p("nums.map double.filter pred"),
        "(call (. (var nums) map) (call (. (var double) filter) (var pred)))"
    );
}

#[test]
fn chain_on_call_result_needs_parens() {
    assert_eq!(
        p("(nums.map double).filter pred"),
        "(call (. (paren (call (. (var nums) map) (var double))) filter) (var pred))"
    );
}

#[test]
fn postfix_bang() {
    assert_eq!(p("print! \"hi\""), r#"(call (! (var print)) (str "hi"))"#);
}

#[test]
fn postfix_question() {
    assert_eq!(p("parseInt s?"), "(call (var parseInt) (? (var s)))");
}

#[test]
fn call_with_lambda_argument() {
    assert_eq!(
        p("nums.map x -> x * 2"),
        "(call (. (var nums) map) (lambda ((pvar x)) (* (var x) (int 2))))"
    );
}
