use demo::format_value;

fn widths() -> Vec<(&'static str, &'static str)> {
    vec![("a", "a  "), ("abcd", "abc"), ("abc", "abc")]
}

#[test]
fn formats_to_the_width() {
    for (value, want) in widths() {
        assert_eq!(format_value(value, 3), want);
    }
}

#[test]
fn pads_and_truncates() {
    assert_eq!(format_value("ab", 3), "ab ");
}
