use demo::format_value;

#[test]
fn keeps_a_value_already_at_the_width() {
    assert_eq!(format_value("abcd", 4), "abcd");
}
