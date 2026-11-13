# TODO(#2013): replace this lock with an asynchronous communication channel
LOCK = []


def send(payload):
    LOCK.append(payload)
    sent = post(payload)
    # TODO: retry once when the queue is full
    LOCK.clear()
    return sent


def post(payload):
    return payload.strip()
