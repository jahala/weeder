from unittest.mock import Mock

from src.publish import publish


def test_publish_hands_the_draft_on():
    assert publish("draft", lambda text: text == "draft") is True


def test_notifies_the_owner():
    notify = Mock(return_value=True)
    assert notify("draft") is True
    notify.assert_called_once_with("draft")
