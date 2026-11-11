from unittest.mock import patch

from src.format import format_value


def test_pads_to_the_width():
    assert format_value("a", 3) == "a  "


def test_format_value_cuts_a_long_value():
    with patch("src.format.format_value", return_value="abc"):
        assert format_value("abcd", 3) == "abc"
