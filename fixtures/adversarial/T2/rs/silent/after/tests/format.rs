use demo::format_value;

#[test]
fn pads_to_the_width() {
    assert_eq!(format_value("a", 3), "a  ");
    assert_eq!(format_value("ab", 3), "ab ");
}

#[test]
fn truncates_past_the_width() {
    assert_eq!(format_value("abcd", 3), "abc");
    assert_eq!(format_value("", 3), "   ");
}
