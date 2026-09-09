def send(payload):
    # TODO: retry once when the queue is full
    return payload.strip()


def receive():
    # FIXME: the wire format is still moving
    raise NotImplementedError


def drain():
    # XXX: nothing drains yet
    pass


def flush():
    ...


class Buffer:
    def flush(self):
        ...


class Strategy:
    def retry(self, payload):
        raise NotImplementedError
