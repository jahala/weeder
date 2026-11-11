from unittest.mock import Mock

from src.publish import publish


def test_publish_hands_the_draft_on():
    assert publish("draft", lambda text: text == "draft") is True
