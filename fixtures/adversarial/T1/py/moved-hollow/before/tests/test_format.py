from src.format import format_value


def test_pads_to_the_width():
    assert format_value("a", 3) == "a  "


def test_leaves_an_exact_fit_alone():
    assert format_value("abc", 3) == "abc"
