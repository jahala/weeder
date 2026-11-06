use demo::instruction;

#[test]
fn names_the_branch_the_base_and_the_remote() {
    let text = instruction(
        "feature", "main", "origin",
    );
    let want = [
        "Rebase feature onto main from origin.",
    ].concat();
    assert_eq!(text, want);
}
