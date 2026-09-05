import pytest
import unittest

from src.format import format_value


def test_pads_to_the_width():
    assert format_value("a", 3) == "a  "


@pytest.mark.skip(reason="the padding is being rewritten")
def test_truncates_past_the_width():
    assert format_value("abcd", 3) == "abc"


@pytest.mark.xfail
def test_leaves_an_exact_fit_alone():
    assert format_value("abc", 3) == "abc"


@unittest.skip("no fill character yet")
def test_pads_with_the_fill_character():
    assert format_value("a", 3) == "a.."
