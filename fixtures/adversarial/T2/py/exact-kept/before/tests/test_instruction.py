from src.instruction import instruction


def test_names_the_branch_the_base_and_the_remote():
    text = instruction(
        "feature", "main", "origin",
    )
    want = "".join([
        "Rebase feature onto main from origin.",
    ])
    assert text == want
    text = instruction(
        "hotfix", "release", "upstream",
    )
    want = "".join([
        "Rebase hotfix onto release from upstream.",
    ])
    assert text == want
