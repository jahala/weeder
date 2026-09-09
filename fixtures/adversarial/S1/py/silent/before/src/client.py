def send(payload):
    return payload.strip()


class Strategy:
    def send(self, payload):
        return send(payload)
