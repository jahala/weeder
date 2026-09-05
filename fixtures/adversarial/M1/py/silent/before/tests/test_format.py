from unittest.mock import patch

from src.clock import now
from src.format import format_value


@patch("src.clock.now")
def test_pads_to_the_width(doubled):
    assert format_value("a", 3) == "a  "
