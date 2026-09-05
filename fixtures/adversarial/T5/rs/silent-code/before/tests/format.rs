use demo::format_value;

#[test]
fn pads_to_the_width() {
    insta::assert_snapshot!(format_value("a", 4));
}
