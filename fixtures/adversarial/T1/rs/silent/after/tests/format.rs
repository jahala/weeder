use demo::format_value;

#[test]
fn pads_the_value_out_to_the_width() {
    assert_eq!(format_value("a", 3), "a  ");
}

#[test]
fn truncates_past_the_width() {
    assert_eq!(format_value("abcd", 3), "abc");
}

#[test]
fn leaves_an_exact_fit_alone() {
    assert_eq!(format_value("abc", 3), "abc");
}
