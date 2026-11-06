use demo::instruction;

#[test]
fn names_the_branch_the_base_and_the_remote() {
    let text = instruction("feature", "main", "origin");
    assert!(text.contains("feature"));
    assert!(text.to_lowercase().contains("main"));
    assert!(text.contains("origin"));
}
