use demo::format_value;

#[test]
fn pads_to_the_width() {
    assert_eq!(format_value("a", 3), "a  ");
}

#[test]
fn leaves_an_exact_fit_alone() {
    assert_eq!(format_value("abc", 3), "abc");
}
