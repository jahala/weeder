import pytest

from src.format import format_value
from tests.widths import WIDTHS


@pytest.mark.parametrize("value,want", WIDTHS)
def test_formats_to_the_width(value, want):
    assert format_value(value, 3) == want


def test_pads_to_the_width():
    assert format_value("a", 3) == "a  "
