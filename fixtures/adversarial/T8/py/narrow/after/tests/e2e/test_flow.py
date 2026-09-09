from src.format import format_value


def test_formats_a_whole_row():
    assert format_value("ab", 4) == "ab  "
