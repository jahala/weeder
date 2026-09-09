from src.client import Strategy


class Fast(Strategy):
    def send(self, payload):
        return payload.strip()

    def retry(self, payload):
        return self.send(payload)
