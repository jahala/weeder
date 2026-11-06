import pytest

from src.format import ratio


class TestRatio:
    def test_divides_to_four_places(self):
        assert ratio(2, 3) == pytest.approx(0.6667, abs=1e-4)
        assert ratio(1, 3) == pytest.approx(0.3333, abs=1e-2)
