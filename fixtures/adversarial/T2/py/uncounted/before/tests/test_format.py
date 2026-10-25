from src.format import format_value


def test_formats_to_the_width():
    assert format_value("a", 3) == "a  "
    assert format_value("abcd", 3) == "abc"
    assert format_value("abc", 3) == "abc"
