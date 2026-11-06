from src.text import slugify


def test_slugify_joins_the_words_with_a_dash():
    assert slugify(" A b ") == "a-b"
