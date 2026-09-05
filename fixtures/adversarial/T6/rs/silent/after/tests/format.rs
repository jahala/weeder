use demo::{parse, Error};

#[test]
fn refuses_an_empty_input() {
    assert_eq!(parse("").unwrap_err(), Error::Empty);
}

#[test]
fn refuses_a_stray_separator() {
    assert_eq!(parse(";;").unwrap_err(), Error::Separator);
}
