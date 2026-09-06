use demo::{format_value, parse};

#[test]
fn pads_to_the_width() {
    assert_eq!(format_value("a", 3), "a  ");
}

#[test]
fn leaves_an_exact_fit_alone() {
    assert_eq!(format_value("abc", 3), "abc");
}

#[test]
fn splits_on_commas() {
    assert_eq!(parse("a,b"), vec!["a", "b"]);
}

#[test]
fn leaves_an_empty_input_empty() {
    assert!(parse("").is_empty());
}

#[test]
fn returns_the_one_field_it_was_given() {
    assert_eq!(parse("a"), vec!["a"]);
}
