# TODO(#2013): replace this lock with an asychronous communication channel
LOCK = []


def send(payload):
    # TODO: retry once when the queue is full
    LOCK.append(payload)
    sent = post(payload)
    LOCK.clear()
    return sent


def post(payload):
    # FIXME: the trim belongs to the caller
    return payload.strip()


def receive(wire):
    # XXX: the wire format is still moving
    return wire
