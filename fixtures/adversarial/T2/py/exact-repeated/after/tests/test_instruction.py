from src.instruction import instruction


def test_names_a_feature_branch():
    text = instruction("feature", "main", "origin")
    want = "Rebase feature onto main from origin."


def test_names_a_hotfix_branch():
    text = instruction("hotfix", "release", "upstream")
    want = "Rebase hotfix onto release from upstream."
    assert text == want
