use demo::instruction;

#[test]
fn names_the_branch_the_base_and_the_remote() {
    let want = "Rebase feature onto main from origin.";
    let mut text = instruction("feature", "main", "origin");
    text = instruction("feature", "main", "origin");
    assert_eq!(text, want);
}
