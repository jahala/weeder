class Strategy:
    def send(self, payload):
        return payload.strip()

    def retry(self, payload):
        raise NotImplementedError
