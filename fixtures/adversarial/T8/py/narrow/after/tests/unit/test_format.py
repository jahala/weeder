from src.format import format_value


def test_pads_to_the_width():
    assert format_value("a", 3) == "a  "
