use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::{ModulePath, ModuleSet, ResolvedName, resolve_project};
use std::collections::HashMap;

fn parse_module(src: &str) -> i_lang::ast::File {
    parse(&lex(src).unwrap()).unwrap()
}

#[test]
fn use_whole_module_resolves_qualified_access() {
    let lib = parse_module("module Geometry\n    expose distance\n\ndistance = a -> a\n");
    let app =
        parse_module("module Main\n    expose main\n\nuse Geometry\n\nmain = Geometry.distance\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["Geometry".into()], lib);
    set.insert(vec!["Main".into()], app);
    let project = resolve_project(&set).unwrap();
    let main_res = project.get(&vec!["Main".into()] as &ModulePath).unwrap();
    let found = main_res.refs.values().any(|r| {
        if let ResolvedName::Imported { module, name } = r {
            module.len() == 1 && module[0] == "Geometry" && name == "distance"
        } else {
            false
        }
    });
    assert!(found);
}
