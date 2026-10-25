use demo::format_value;

#[test]
fn formats_to_the_width() {
    assert_eq!(format_value("a", 3), "a  ");
    assert_eq!(format_value("abcd", 3), "abc");
    assert_eq!(format_value("abc", 3), "abc");
}
