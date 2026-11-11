from src.banner import banner


def test_banner_is_a_paragraph():
    assert banner(LookupError("x")).startswith("<p>")


def test_banner_shows_the_error_it_was_given():
    error = LookupError("Workspace not found")
    assert "Workspace not found" in banner(error)
