def send(payload):
    return payload.strip()


def label(kind):
    if kind == "todo":
        return "TODO: written by the caller"
    return "done"


def version():
    return "1.4.0"
