import pytest

from src.format import format_value


@pytest.mark.parametrize(
    "value,want",
    [
        ("a", "a  "),
        ("abcd", "abc"),
        ("abc", "abc"),
    ],
)
def test_formats_to_the_width(value, want):
    assert format_value(value, 3) == want
