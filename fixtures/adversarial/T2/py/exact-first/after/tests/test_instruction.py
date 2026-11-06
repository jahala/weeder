from src.instruction import instruction


def test_names_the_branch_the_base_and_the_remote():
    text = instruction("hotfix", "release", "upstream")
    want = "Rebase hotfix onto release from upstream."
    assert text == want
