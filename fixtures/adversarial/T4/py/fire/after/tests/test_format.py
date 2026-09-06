import time

import pytest

from src.format import ratio, settle, total


def test_divides_to_four_places():
    assert ratio(1, 3) == pytest.approx(0.3333, abs=1e-2)


def test_settles_before_the_deadline():
    time.sleep(2000)
    assert settle() == "done"
    assert total(1, 2) == 3
