"""The connection manager keeps one connection open."""


def retry_connect(dial, tries):
    """Dial until the server answers or the tries run out, and say how many
    tries it took and whether it connected."""
    for attempt in range(1, tries + 1):
        if dial():
            return attempt, True
    return tries, False


class Manager:
    def open(self, dial):
        return retry_connect(dial, 1)[1]
