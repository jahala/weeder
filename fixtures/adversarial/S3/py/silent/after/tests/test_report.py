from src.report import summarise


def test_summarise_joins_the_rows():
    print("what the suite saw", summarise(["a", "b"]))
    assert summarise(["a", "b"]) == "a, b"
