use demo::parse;

#[test]
fn splits_on_commas() {
    assert_eq!(parse("a,b"), vec!["a", "b"]);
}

#[test]
fn leaves_an_empty_input_empty() {
    assert!(parse("").is_empty());
}
