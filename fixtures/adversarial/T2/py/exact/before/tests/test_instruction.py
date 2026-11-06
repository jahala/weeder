from src.instruction import instruction


def test_names_the_branch_the_base_and_the_remote():
    text = instruction("feature", "main", "origin")
    assert "feature" in text
    assert "main" in text.lower()
    assert "origin" in text
