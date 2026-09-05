def send(payload):
    try:
        return post(payload)
    except Exception:
        pass
    return ""


def receive():
    try:
        return read()
    except:
        pass
    return ""


def post(payload):
    return payload.strip()


def read():
    return "ready"
