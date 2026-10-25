use demo::format_value;

#[test]
fn formats_to_the_width() {
    for (value, want) in [("a", "a  "), ("abcd", "abc")] {
        assert_eq!(format_value(value, 3), want);
    }
}
