use demo::format_value;
use rstest::rstest;

#[rstest]
#[case("a", "a  ")]
#[case("abcd", "abc")]
fn formats_to_the_width(#[case] value: &str, #[case] want: &str) {
    assert_eq!(format_value(value, 3), want);
}
