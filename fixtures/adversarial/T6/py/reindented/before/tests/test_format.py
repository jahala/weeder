import pytest

from src.format import SeparatorError, parse


def test_rejects_bad_input():
    pytest.raises(Exception, parse, "")
    pytest.raises(SeparatorError, parse, ";;")
