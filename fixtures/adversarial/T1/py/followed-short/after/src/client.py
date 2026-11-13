"""The client talks to the server."""


def retry_connect(dial, tries):
    """Dial until the server answers or the tries run out, and say how many
    tries it took and whether it connected."""
    for attempt in range(1, tries + 1):
        if dial():
            return attempt, True
    return tries, False


class Client:
    def connect(self, dial):
        return retry_connect(dial, 3)[1]
