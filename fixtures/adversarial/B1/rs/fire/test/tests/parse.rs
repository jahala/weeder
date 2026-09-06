#[test]
fn splits_on_commas() {
    assert_eq!(records::parse("a,b"), vec!["a", "b"]);
}

#[test]
fn keeps_an_empty_field() {
    assert_eq!(records::parse("a,,b"), vec!["a", "", "b"]);
}

#[test]
fn keeps_a_single_field_whole() {
    assert_eq!(records::parse("a"), vec!["a"]);
}
