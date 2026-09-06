from src.format import format_value


def test_pads_to_the_width():
    assert format_value("a", 3) == "a  "


def test_truncates_past_the_width():
    assert format_value("abcd", 3) == "abc"


def test_leaves_a_value_of_the_width_alone():
    assert format_value("abc", 3) == "abc"


def test_pads_an_empty_value_out_to_the_width():
    assert format_value("", 3) == "   "
