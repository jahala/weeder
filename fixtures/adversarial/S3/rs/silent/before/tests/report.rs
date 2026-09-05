#[test]
fn summarise_joins_the_rows() {
    assert_eq!(demo::report::summarise(&["a".to_string()]), "a");
}
