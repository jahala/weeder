use format::format;

#[test]
fn pads_to_the_width() {
    assert_eq!(format("a", 3), "a  ");
}
