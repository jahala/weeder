from src.format import format_value

WIDTHS = [("a", "a  "), ("abcd", "abc"), ("abc", "abc")]


def test_formats_to_the_width():
    def check(case):
        value, want = case
        assert format_value(value, 3) == want

    list(map(check, WIDTHS))
