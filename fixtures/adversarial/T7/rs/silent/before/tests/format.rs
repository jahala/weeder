use demo::format_value;

#[test]
fn pads_to_the_width() {
    assert_eq!(format_value("a", 3), "a  ");
}

#[test]
fn truncates_past_the_width() {
    assert_eq!(format_value("abcd", 3), "abc");
}
