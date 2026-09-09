from abc import ABC
from typing import Protocol


class Sink(Protocol):
    def send(self, payload: str) -> str: ...

    def close(self) -> None: ...


class Store(ABC):
    def read(self, key: str) -> str: ...


def send(payload):
    return payload.strip()


def label(kind):
    if kind == "todo":
        return "TODO: written by the caller"
    return "done"


def version():
    return "1.4.0"


class Strategy:
    def send(self, payload):
        return send(payload)

    def retry(self, payload):
        raise NotImplementedError
