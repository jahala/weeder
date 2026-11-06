use demo::instruction;

#[test]
fn names_a_feature_branch() {
    let text = instruction("feature", "main", "origin");
    let want = "Rebase feature onto main from origin.";
    assert_eq!(text, want);
}

#[test]
fn names_a_hotfix_branch() {
    let text = instruction("hotfix", "release", "upstream");
    let want = "Rebase hotfix onto release from upstream.";
    assert_eq!(text, want);
}
