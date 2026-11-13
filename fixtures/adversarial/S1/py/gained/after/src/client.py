# TODO(#2013): replace this lock with an asynchronous communication channel
LOCK = []


def send(payload):
    LOCK.append(payload)
    sent = post(payload)
    # TODO: retry once when the queue is full
    LOCK.clear()
    return sent


def post(payload):
    # FIXME: the trim belongs to the caller. FIXME: so does the pad
    return payload.strip()


def receive(wire):
    # FIXME: the wire format is still moving
    # TODO: close the socket after the last frame
    return wire
