use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::pretty::pretty;

fn rt(src: &str) -> String {
    let ast = parse(&lex(src).unwrap()).unwrap();
    pretty(&ast)
}

#[test]
fn simple_binding() {
    assert!(rt("x = 1\n").contains("x = 1"));
}

#[test]
fn lambda() {
    assert!(rt("add = a b -> a + b\n").contains("a b -> a + b"));
}

#[test]
fn type_block() {
    let src = "type Point\n    x : Float\n    y : Float\n";
    let out = rt(src);
    assert!(out.contains("type Point"));
    assert!(out.contains("x : Float"));
}
