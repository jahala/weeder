use demo::{ratio, settle, total};
use std::time::Duration;

#[test]
fn divides_to_four_places() {
    assert!((ratio(1.0, 3.0) - 0.3333).abs() < 1e-6);
}

#[test]
fn settles_before_the_deadline() {
    assert_eq!(settle(Duration::from_millis(50)), "done");
    assert_eq!(total(4, 5), 9);
}
