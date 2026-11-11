use demo::clock::Clock;
use demo::width::{Counted, Width};

mockall::mock! {
    pub Width {}
    impl Width for Width {
        fn of(&self, value: &str) -> usize;
    }
}

mockall::mock! {
    pub Clock {}
    impl Clock for Clock {
        fn now(&self) -> u64;
    }
}

#[test]
fn counted_reads_the_length() {
    assert_eq!(Counted.of("ab"), 2);
}

#[test]
fn width_counts_the_characters() {
    let mut doubled = MockWidth::new();
    doubled.expect_of().return_const(2_usize);
    assert_eq!(Counted.of("ab"), 2);
}
