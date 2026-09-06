use demo::format_value;

#[test]
fn leaves_an_exact_fit_alone() {
    assert_eq!(format_value("abc", 3), "abc");
}

#[test]
fn pads_to_the_width() {
    let padded = format_value("a", 3);
    assert_eq!(padded, "a  ");
    assert_eq!(format_value("ab", 3), "ab ");
}

#[test]
fn cuts_a_long_value_down() {
    assert_eq!(format_value("abcde", 4), "abcd");
}
