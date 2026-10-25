from src.format import format_value


def widths():
    return [("a", "a  "), ("abcd", "abc"), ("abc", "abc")]


def test_formats_to_the_width():
    for value, want in widths():
        assert format_value(value, 3) == want


def test_pads_and_truncates():
    assert format_value("ab", 3) == "ab "
