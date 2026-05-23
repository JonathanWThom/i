use i_lang::error::ErrorKind;
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

#[test]
fn use_alias_resolves_qualified() {
    let lib = parse_module("module Std.Float\n    expose parse\n\nparse = x -> x\n");
    let app =
        parse_module("module Main\n    expose main\n\nuse Std.Float as F\n\nmain = F.parse\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["Std".into(), "Float".into()], lib);
    set.insert(vec!["Main".into()], app);
    let project = resolve_project(&set).unwrap();
    let main_res = project.get(&vec!["Main".into()] as &ModulePath).unwrap();
    let found = main_res.refs.values().any(|r| {
        if let ResolvedName::Imported { module, name } = r {
            module.len() == 2 && module[0] == "Std" && module[1] == "Float" && name == "parse"
        } else {
            false
        }
    });
    assert!(found);
}

#[test]
fn aliased_original_path_is_unavailable() {
    let lib = parse_module("module Std.Float\n    expose parse\n\nparse = x -> x\n");
    let app = parse_module(
        "module Main\n    expose main\n\nuse Std.Float as F\n\nmain = Std.Float.parse\n",
    );
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["Std".into(), "Float".into()], lib);
    set.insert(vec!["Main".into()], app);
    let errs = resolve_project(&set).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(&e.kind, ErrorKind::Unresolved { .. }))
    );
}

#[test]
fn use_cherry_pick_brings_names_unqualified() {
    let lib = parse_module(
        "module Geometry\n    expose Point, distance\n\ntype Point\n    x : Point\n\ndistance = a -> a\n",
    );
    let app = parse_module(
        "module Main\n    expose main\n\nuse Geometry (Point, distance)\n\nmain = distance\n",
    );
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

#[test]
fn cherry_pick_unexposed_is_error() {
    let lib = parse_module(
        "module Geometry\n    expose distance\n\nsquare = x -> x\ndistance = a -> a\n",
    );
    let app =
        parse_module("module Main\n    expose main\n\nuse Geometry (square)\n\nmain = square\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["Geometry".into()], lib);
    set.insert(vec!["Main".into()], app);
    let errs = resolve_project(&set).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::NotExposed { name, .. } if name == "square"
    )));
}

#[test]
fn module_cycle_detected() {
    let a = parse_module("module A\n    expose x\n\nuse B\n\nx = 1\n");
    let b = parse_module("module B\n    expose y\n\nuse A\n\ny = 2\n");
    let mut set: ModuleSet = HashMap::new();
    set.insert(vec!["A".into()], a);
    set.insert(vec!["B".into()], b);
    let errs = resolve_project(&set).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(&e.kind, ErrorKind::ModuleCycle { .. }))
    );
}
