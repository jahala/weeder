use demo::{ratio, settle, total};
use std::time::Duration;

#[test]
fn divides_to_four_places() {
    assert!((ratio(1.0, 3.0) - 0.3333).abs() < 1e-2);
}

#[test]
fn settles_before_the_deadline() {
    assert_eq!(settle(Duration::from_millis(2000)), "done");
    assert_eq!(total(1, 2), 3);
}
