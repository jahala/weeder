"""The connection manager keeps one connection open."""


class Manager:
    def open(self, dial):
        return dial()
