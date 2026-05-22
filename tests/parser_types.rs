use i_lang::lex::lex;
use i_lang::parse::parse_type_for_test;

fn t(src: &str) -> String {
    format!("{}", parse_type_for_test(&lex(src).unwrap()).unwrap().node)
}

#[test]
fn type_var() {
    assert_eq!(t("a"), "(tvar a)");
}

#[test]
fn named_type() {
    assert_eq!(t("Int"), "(tnamed Int)");
}

#[test]
fn parametric() {
    assert_eq!(t("List a"), "(tnamed List (tvar a))");
}

#[test]
fn function_type() {
    assert_eq!(
        t("Int, Int -> Int"),
        "(tfun (tnamed Int) (tnamed Int) (tnamed Int))"
    );
}

#[test]
fn effectful_type() {
    assert_eq!(
        t("String ! IO -> Unit"),
        "(tfun (tnamed String) (eff IO) (tnamed Unit))"
    );
}

#[test]
fn empty_effect_row() {
    assert_eq!(t("(a -> b ! ())"), "(tfun (tvar a) (eff-empty) (tvar b))");
}
