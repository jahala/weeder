use demo::width::Width;

mockall::mock! {
    pub Width {}
    impl Width for Width {
        fn of(&self, value: &str) -> usize;
    }
}

#[test]
fn reads_what_the_double_reports() {
    let mut doubled = MockWidth::new();
    doubled.expect_of().return_const(2_usize);
    assert_eq!(doubled.of("ab"), 2);
}
