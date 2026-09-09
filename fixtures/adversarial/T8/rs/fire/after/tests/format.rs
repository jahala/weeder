#![cfg(feature = "slow")]

use format::format;

#[test]
fn pads_to_the_width() {
    assert_eq!(format("a", 3), "a  ");
}

#[test]
fn truncates_past_the_width() {
    assert_eq!(format("abcd", 3), "abc");
}
