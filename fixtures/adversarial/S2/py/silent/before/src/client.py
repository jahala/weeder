def send(payload):
    return post(payload)


def receive():
    return read()


def post(payload):
    return payload.strip()


def read():
    return "ready"
