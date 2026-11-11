import pytest

from src.parse import normalise, parse


def trims_to(text, expected):
    assert normalise(text) == expected


def test_reads_a_number():
    assert parse("42") == 42


def test_refuses_an_empty_string():
    with pytest.raises(ValueError, match="empty input"):
        parse("")


def test_trims_a_padded_number():
    trims_to(" 7 ", "7")
