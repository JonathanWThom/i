use i_lang::lex::lex;
use i_lang::parse::parse;

fn p(src: &str) -> String {
    format!("{}", parse(&lex(src).unwrap()).unwrap())
}

#[test]
fn sig_only() {
    assert_eq!(p("x : Int"), "(file\n  (sig x (tnamed Int)))");
}

#[test]
fn annotated_value() {
    assert_eq!(p("x : Int = 1"), "(file\n  (let x : (tnamed Int) (int 1)))");
}

#[test]
fn value_only() {
    assert_eq!(p("x = 1"), "(file\n  (let x (int 1)))");
}

#[test]
fn block_body() {
    let src = "x =\n    1\n    2";
    assert_eq!(
        p(src),
        "(file\n  (let x (block\n    (int 1)\n    (int 2))))"
    );
}

#[test]
fn nested_binding_in_block() {
    let src = "x =\n    y = 2\n    y + 1";
    assert_eq!(
        p(src),
        "(file\n  (let x (block\n    (let y (int 2))\n    (+ (var y) (int 1)))))"
    );
}

#[test]
fn two_top_level_bindings() {
    assert_eq!(
        p("x = 1\ny = 2"),
        "(file\n  (let x (int 1))\n  (let y (int 2)))"
    );
}
