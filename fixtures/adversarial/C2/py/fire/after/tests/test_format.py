from src.format import format_fields


def test_joins_the_fields():
    assert format_fields(["a", "b"]) == "a,b"
