from src.format import format_value


def test_pads_to_the_width(snapshot):
    assert format_value("a", 4) == snapshot
