use demo::instruction;

#[test]
fn names_the_branch_the_base_and_the_remote() {
    let text = instruction("feature", "main", "origin");
    assert_eq!(text.len(), 37);
}
