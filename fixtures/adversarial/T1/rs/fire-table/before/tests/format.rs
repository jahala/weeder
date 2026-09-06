use demo::format_value;

#[test]
fn pads_to_the_width() {
    assert_eq!(format_value("a", 3), "a  ");
}

#[test]
fn truncates_past_the_width() {
    assert_eq!(format_value("abcd", 3), "abc");
}

#[test]
fn leaves_a_value_of_the_width_alone() {
    assert_eq!(format_value("abc", 3), "abc");
}

#[test]
fn pads_an_empty_value_out_to_the_width() {
    assert_eq!(format_value("", 3), "   ");
}
