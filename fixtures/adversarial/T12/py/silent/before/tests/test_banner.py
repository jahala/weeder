from src.banner import banner


def test_banner_is_a_paragraph():
    assert banner(LookupError("x")).startswith("<p>")
