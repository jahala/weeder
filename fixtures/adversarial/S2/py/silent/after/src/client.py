import logging

log = logging.getLogger(__name__)


def send(payload):
    try:
        return post(payload)
    except ValueError as error:
        log.warning("send refused %r: %s", payload, error)
        return ""


def receive():
    try:
        return read()
    except OSError as error:
        raise RuntimeError("receive failed") from error


def drain():
    try:
        return read()
    except OSError:
        # the socket is already closed, and a closed socket has nothing left
        pass


def post(payload):
    return payload.strip()


def read():
    return "ready"
