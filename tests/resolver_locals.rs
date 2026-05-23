use i_lang::error::ErrorKind;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::{ResolvedName, resolve_file};

#[test]
fn var_resolves_to_top_level() {
    let src = "module M\n    expose y\n\nx = 1\ny = x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let x_resolutions: Vec<_> = res
        .refs
        .values()
        .filter(|r| matches!(r, ResolvedName::TopLevel(_)))
        .collect();
    assert_eq!(x_resolutions.len(), 1);
}

#[test]
fn unknown_var_is_error() {
    let src = "module M\n    expose y\n\ny = unknownName\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "unknownName"
    )));
}

#[test]
fn ctor_resolves() {
    let src = "module M\n    expose Shape\n\ntype Shape\n    Circle\n        radius : Float\n    Rect\n        width : Float\n        height : Float\n\nmkCircle = Circle(radius = 1.0)\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    use i_lang::resolve::DefKind;
    assert!(
        res.defs
            .iter()
            .any(|d| d.name == "Circle" && matches!(d.kind, DefKind::Ctor { .. }))
    );
    let ctor_refs: Vec<_> = res
        .refs
        .values()
        .filter(|r| matches!(r, ResolvedName::Ctor(_)))
        .collect();
    assert_eq!(ctor_refs.len(), 1);
}

#[test]
fn lambda_param_shadows_top_level() {
    let src = "module M\n    expose f\n\nx = 1\nf = x -> x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let local_refs: Vec<_> = res
        .refs
        .values()
        .filter(|r| matches!(r, ResolvedName::Local(_)))
        .collect();
    assert_eq!(local_refs.len(), 1);
}

#[test]
fn duplicate_lambda_param_is_error() {
    let src = "module M\n    expose f\n\nf = x x -> x\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::DuplicateLocal { name } if name == "x"
    )));
}

#[test]
fn match_arm_binds_pattern_vars() {
    let src = "module M\n    expose f\n\ntype Option a\n    None\n    Some : a\n\nf = x ->\n    x match\n        Some y -> y\n        None -> 0\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let local_refs: Vec<_> = res
        .refs
        .values()
        .filter(|r| matches!(r, ResolvedName::Local(_)))
        .collect();
    // x (scrutinee) + y (arm body) = 2 Local refs.
    assert_eq!(local_refs.len(), 2);
}

#[test]
fn pattern_var_out_of_arm_is_error() {
    let src = "module M\n    expose f\n\ntype Option a\n    None\n    Some : a\n\nf = x ->\n    z = x match\n        Some y -> y\n        None -> 0\n    y\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "y"
    )));
}

#[test]
fn block_let_binding_visible_later() {
    let src = "module M\n    expose f\n\nf = x ->\n    a = x + 1\n    b = a + 1\n    b\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let local_refs: Vec<_> = res
        .refs
        .values()
        .filter(|r| matches!(r, ResolvedName::Local(_)))
        .collect();
    // x (in a = x+1), a (in b = a+1), b (final expr) = 3.
    assert_eq!(local_refs.len(), 3);
}

#[test]
fn block_let_binding_not_visible_earlier() {
    let src = "module M\n    expose f\n\nf =\n    a = b\n    b = 1\n    a\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "b"
    )));
}
