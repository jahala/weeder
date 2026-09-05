import pytest

from src.format import parse, SeparatorError


def test_refuses_an_empty_input():
    with pytest.raises(Exception):
        parse("")


def test_refuses_a_stray_separator():
    with pytest.raises(Exception):
        parse(";;")
