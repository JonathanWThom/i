use i_lang::check::check_file;
use i_lang::lex::lex;
use i_lang::parse::parse;
use i_lang::resolve::resolve_file;

#[test]
fn newtype_declaration_is_nominal() {
    // A newtype is a distinct nominal type. `firstUser : UserId` and
    // `firstUser = 1` should TypeMismatch — `1 : Int` doesn't unify with UserId
    // even though UserId is "the same as" Int structurally.
    let src = "\
type UserId = Int
firstUser : UserId
firstUser = 1
";
    let file = parse(&lex(src).unwrap()).unwrap();
    let res = resolve_file(&file).unwrap();
    let errs = check_file(&file, &res).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e.kind, i_lang::error::ErrorKind::TypeMismatch { .. }))
    );
}
