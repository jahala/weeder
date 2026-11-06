import pytest

from src.format import SeparatorError, parse


class TestParse:
    def test_rejects_bad_input(self):
        pytest.raises(Exception, parse, "")
        pytest.raises(SeparatorError, parse, ";;")
