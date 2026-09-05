#[test]
fn summarise_joins_the_rows() {
    println!("what the suite saw");
    assert_eq!(demo::report::summarise(&["a".to_string()]), "a");
}
