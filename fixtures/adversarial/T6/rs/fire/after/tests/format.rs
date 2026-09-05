use demo::{parse, Error};

#[test]
fn refuses_an_empty_input() {
    assert!(parse("").is_err());
}

#[test]
fn refuses_a_stray_separator() {
    assert!(parse(";;").is_err());
}
