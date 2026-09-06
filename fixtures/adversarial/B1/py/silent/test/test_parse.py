import unittest

from parse import parse


class TestParse(unittest.TestCase):
    def test_splits_on_commas(self):
        self.assertEqual(parse("a,b"), ["a", "b"])

    def test_splits_on_semicolons(self):
        self.assertEqual(parse("a;b"), ["a", "b"])
