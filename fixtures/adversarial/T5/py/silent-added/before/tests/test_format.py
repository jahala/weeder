from src.format import format_value


def test_keeps_a_value_already_at_the_width():
    assert format_value("abcd", 4) == "abcd"
