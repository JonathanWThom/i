use i_lang::check::check_file;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn sum_type_with_variants_is_registered() {
    let src = "\
type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let typing = check_file(&file, &res).expect("expected check to succeed");
    let circle = res.defs.iter().find(|d| d.name == "Circle").unwrap();
    let rect = res.defs.iter().find(|d| d.name == "Rect").unwrap();
    assert!(typing.schemes.contains_key(&circle.id));
    assert!(typing.schemes.contains_key(&rect.id));
}
