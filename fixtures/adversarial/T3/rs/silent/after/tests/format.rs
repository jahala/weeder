use demo::format_value;

#[test]
fn pads_to_the_width() {
    assert_eq!(format_value("a", 3), "a  ");
}

// #[ignore] came off this case when the padding stopped rounding.
#[test]
fn truncates_past_the_width() {
    assert_eq!(format_value("abcd", 3), "abc");
}

#[test]
fn names_the_marker_a_reviewer_greps_for() {
    assert_eq!(marker(), "#[ignore]");
}
