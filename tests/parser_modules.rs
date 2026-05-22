use i_lang::ast::Expose;
use i_lang::lex::lex;
use i_lang::parse::parse;

fn p(src: &str) -> String {
    format!("{}", parse(&lex(src).unwrap()).unwrap())
}

#[test]
fn module_header() {
    let src = "module Main\n    expose main, helper\n";
    let f = parse(&lex(src).unwrap()).unwrap();
    assert_eq!(f.module.unwrap().exposes.len(), 2);
}

#[test]
fn use_whole() {
    assert_eq!(p("use Std.IO"), "(file\n  (use Std.IO whole))");
}

#[test]
fn use_cherry() {
    assert_eq!(
        p("use Std.IO (print, readLine)"),
        "(file\n  (use Std.IO (cherry print readLine)))"
    );
}

#[test]
fn use_alias() {
    assert_eq!(
        p("use Std.Float as F"),
        "(file\n  (use Std.Float (alias F)))"
    );
}

#[test]
fn expose_with_constructors() {
    let src = "module Geometry\n    expose Point(..), distance\n";
    let f = parse(&lex(src).unwrap()).unwrap();
    let exp = &f.module.unwrap().exposes;
    assert!(matches!(
        &exp[0],
        Expose::Type {
            with_constructors: true,
            ..
        }
    ));
    assert!(matches!(&exp[1], Expose::Value(_)));
}

#[test]
fn module_then_decls() {
    let src = "module Main\n    expose main\nmain = 1";
    assert_eq!(
        p(src),
        "(file\n  (module Main (expose-val main))\n  (let main (int 1)))"
    );
}
