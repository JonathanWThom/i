use i_lang::error::ErrorKind;
use i_lang::lex::lex;

#[test]
fn mixed_tabs_and_spaces_in_indent() {
    // Tab then space on the same indent run.
    let src = "main =\n\t x\n";
    let err = lex(src).unwrap_err();
    assert!(
        matches!(err.kind, ErrorKind::MixedTabsAndSpaces),
        "got {:?}",
        err.kind
    );
}

#[test]
fn tabs_after_spaces_file_is_error() {
    // First indented line uses spaces; later one uses tabs. File must commit.
    let src = "a =\n    b\nb =\n\tc\n";
    let err = lex(src).unwrap_err();
    assert!(
        matches!(err.kind, ErrorKind::MixedTabsAndSpaces),
        "got {:?}",
        err.kind
    );
}

#[test]
fn spaces_only_ok() {
    let src = "main =\n    x\n";
    assert!(lex(src).is_ok());
}

#[test]
fn tabs_only_ok() {
    let src = "main =\n\tx\n";
    assert!(lex(src).is_ok());
}
