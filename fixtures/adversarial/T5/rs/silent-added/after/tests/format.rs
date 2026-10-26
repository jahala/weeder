use demo::format_value;

const PADDED: &str = include_str!("fixtures/padded.txt");

#[test]
fn keeps_a_value_already_at_the_width() {
    assert_eq!(format_value("abcd", 4), "abcd");
}

#[test]
fn pads_a_short_value_with_dots() {
    assert_eq!(format_value("a", 4), PADDED.trim_end());
}
