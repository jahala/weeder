use demo::format_value;

const WIDTHS: [(&str, &str); 3] = [("a", "a  "), ("abcd", "abc"), ("abc", "abc")];

#[test]
fn formats_to_the_width() {
    WIDTHS.iter().for_each(|(value, want)| {
        assert_eq!(format_value(value, 3), *want);
    });
}
