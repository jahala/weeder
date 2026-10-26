import json
from pathlib import Path

from src.format import format_value

PADDED = json.loads((Path(__file__).parent / "fixtures" / "padded.json").read_text())


def test_keeps_a_value_already_at_the_width():
    assert format_value("abcd", 4) == "abcd"


def test_pads_a_short_value_with_dots():
    assert format_value(PADDED["value"], PADDED["width"]) == PADDED["padded"]
