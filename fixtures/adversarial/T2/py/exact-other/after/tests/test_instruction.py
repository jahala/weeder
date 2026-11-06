from src.instruction import instruction


def test_names_the_branch_the_base_and_the_remote():
    text = instruction("feature", "main", "origin")
    assert len(text) == 37
