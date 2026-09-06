use demo::clock::Clock;

mockall::mock! {
    pub Clock {}
    impl Clock for Clock {
        fn now(&self) -> u64;
    }
}

#[test]
fn reads_what_the_double_reports() {
    let mut doubled = MockClock::new();
    doubled.expect_now().return_const(0_u64);
    assert_eq!(doubled.now(), 0);
}
