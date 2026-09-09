use format::format;

#[test]
fn truncates_past_the_width() {
    assert_eq!(format("abcd", 3), "abc");
}
