from src.parse import normalise, parse


def test_reads_a_number():
    assert parse("42") == 42


def test_trims_a_padded_number():
    normalise(" 7 ")
