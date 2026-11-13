from src.client import Client, retry_connect


def dialler(failures):
    """A dial that answers after the given number of refusals."""
    calls = []

    def dial():
        calls.append(None)
        return len(calls) > failures

    return dial


def test_the_client_connects():
    assert Client().connect(lambda: True)


def test_the_client_keeps_dialling():
    _, answered = retry_connect(dialler(2), 5)
    assert answered, "a server that answers on the third try was not reached"
