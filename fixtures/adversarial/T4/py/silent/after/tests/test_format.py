import time

import pytest

from src.format import ratio, settle, total


def test_divides_to_four_places():
    assert ratio(1, 3) == pytest.approx(0.3333, abs=1e-6)


def test_settles_before_the_deadline():
    time.sleep(50)
    assert settle() == "done"
    assert total(4, 5) == 9
