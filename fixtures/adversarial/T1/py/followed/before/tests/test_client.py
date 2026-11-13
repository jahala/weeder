from src.client import Client


def test_the_client_connects():
    assert Client().connect(lambda: True)
