from src.report import summarise


def test_summarise_joins_the_rows():
    assert summarise(["a", "b"]) == "a, b"
