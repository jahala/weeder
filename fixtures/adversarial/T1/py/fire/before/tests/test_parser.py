from src.parser import parse


def test_splits_on_commas():
    assert parse("a,b") == ["a", "b"]


def test_leaves_an_empty_input_empty():
    assert parse("") == []
