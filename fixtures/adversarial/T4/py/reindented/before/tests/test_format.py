import pytest

from src.format import ratio


def test_divides_to_four_places():
    assert ratio(2, 3) == pytest.approx(0.6667, abs=1e-4)
    assert ratio(1, 3) == pytest.approx(0.3333, abs=1e-4)
