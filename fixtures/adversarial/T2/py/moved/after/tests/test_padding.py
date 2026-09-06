from src.format import format_value


def test_leaves_an_exact_fit_alone():
    assert format_value("abc", 3) == "abc"


def test_pads_to_the_width():
    padded = format_value("a", 3)
    assert padded == "a  "
    assert format_value("ab", 3) == "ab "


def test_cuts_a_long_value_down():
    assert format_value("abcde", 4) == "abcd"
