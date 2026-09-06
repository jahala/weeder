from src.format import format_value


def test_leaves_an_exact_fit_alone():
    assert format_value("abc", 3) == "abc"
