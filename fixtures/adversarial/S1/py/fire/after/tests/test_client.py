from src.client import Strategy


class Recording(Strategy):
    def retry(self, payload):
        return payload


def test_records_the_payload():
    assert Recording().retry("a") == "a"
