import pytest

from src.format import parse, SeparatorError


def test_refuses_an_empty_input():
    with pytest.raises(ValueError, match="input is empty"):
        parse("")


def test_refuses_a_stray_separator():
    with pytest.raises(SeparatorError):
        parse(";;")
