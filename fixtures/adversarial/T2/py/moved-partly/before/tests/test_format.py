from src.format import format_value


def test_pads_to_the_width():
    assert format_value("a", 3) == "a  "
    assert format_value("ab", 3) == "ab "


def test_truncates_past_the_width():
    assert format_value("abcd", 3) == "abc"
    assert format_value("abcde", 4) == "abcd"
    assert format_value("abcdef", 2) == "ab"
