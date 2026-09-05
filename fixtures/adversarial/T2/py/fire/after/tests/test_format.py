from src.format import format_value


def test_pads_to_the_width():
    assert format_value("a", 3) == "a  "


def test_truncates_past_the_width():
    assert format_value("abcd", 3) == "abc"
