from src.parse import normalise, parse


def test_reads_a_number():
    assert parse("42") == 42
