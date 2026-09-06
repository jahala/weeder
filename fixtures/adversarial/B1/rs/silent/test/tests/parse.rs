#[test]
fn splits_on_commas() {
    assert_eq!(records::parse("a,b"), vec!["a", "b"]);
}

#[test]
fn splits_on_semicolons() {
    assert_eq!(records::parse("a;b"), vec!["a", "b"]);
}
