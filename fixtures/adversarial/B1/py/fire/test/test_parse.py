import unittest

from parse import parse


class TestParse(unittest.TestCase):
    def test_splits_on_commas(self):
        self.assertEqual(parse("a,b"), ["a", "b"])

    def test_keeps_an_empty_field(self):
        self.assertEqual(parse("a,,b"), ["a", "", "b"])

    def test_keeps_a_single_field_whole(self):
        self.assertEqual(parse("a"), ["a"])
