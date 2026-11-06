from src.instruction import instruction


def test_names_the_branch_the_base_and_the_remote():
    want = "Rebase feature onto main from origin."
    text = instruction("feature", "main", "origin")
    assert text == want
    again = instruction("feature", "main", "origin")
    assert again == want
