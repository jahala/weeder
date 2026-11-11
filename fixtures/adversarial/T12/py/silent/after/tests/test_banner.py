from src.banner import banner


def test_banner_is_a_paragraph():
    assert banner(LookupError("x")).startswith("<p>")


def test_banner_wraps_the_error_in_a_paragraph():
    error = LookupError("Workspace not found")
    assert banner(error) == "<p>Workspace not found</p>"
