from src.format import format_value


def test_formats_to_the_width():
    for value, want in [("a", "a  "), ("abcd", "abc"), ("abc", "abc")]:
        assert format_value(value, 3) == want
