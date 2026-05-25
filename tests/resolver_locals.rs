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
    let src = "module M\n    expose Shape\n\ntype Float\n    v : Float\n\ntype Shape\n    Circle\n        radius : Float\n    Rect\n        width : Float\n        height : Float\n\nmkCircle = Circle(radius = 1.0)\n";
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
    // x (lambda param binding) + x (body use) = 2 Local refs.
    assert_eq!(local_refs.len(), 2);
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
    // x (lambda param binding + scrutinee use) + y (pattern binding + arm body use) = 4.
    assert_eq!(local_refs.len(), 4);
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
    // x (param binding + use in a=x+1), a (binding + use in b=a+1), b (binding + final expr) = 6.
    assert_eq!(local_refs.len(), 6);
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

#[test]
fn self_resolves_in_method() {
    let src = "module M\n    expose Point\n\ntype Float\n    v : Float\n\ntype Point\n    x : Float\n    y : Float\n    sumXY = self.x + self.y\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let self_refs: Vec<_> = res
        .refs
        .values()
        .filter(|r| matches!(r, ResolvedName::Local(_)))
        .collect();
    // self binding site (recorded at the method's decl.span) + 2 uses = 3.
    assert_eq!(self_refs.len(), 3);
}

#[test]
fn self_not_in_scope_outside_method() {
    let src = "module M\n    expose f\n\nf = self\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "self"
    )));
}

#[test]
fn type_in_annotation_unresolved() {
    let src = "module M\n    expose Point\n\ntype Point\n    x : Frobnication\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let errs = resolve_file(&file).unwrap_err();
    assert!(errs.iter().any(|e| matches!(
        &e.kind,
        ErrorKind::Unresolved { name } if name == "Frobnication"
    )));
}

#[test]
fn known_type_resolves_in_annotation() {
    let src = "module M\n    expose pi\n\ntype Foo\n    dummy : Foo\n\npi : Foo\npi = 3\n";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    // Two refs to Foo: one in the field type, one in pi's annotation.
    let type_refs: Vec<_> = res
        .refs
        .values()
        .filter(|r| matches!(r, ResolvedName::TopLevel(_)))
        .collect();
    assert_eq!(type_refs.len(), 2);
}
