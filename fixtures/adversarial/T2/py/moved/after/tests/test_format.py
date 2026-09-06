from src.format import format_value


def test_truncates_past_the_width():
    assert format_value("abcd", 3) == "abc"
