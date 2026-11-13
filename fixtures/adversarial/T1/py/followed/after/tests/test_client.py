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
    first, answered = retry_connect(dialler(0), 3)
    assert answered, "a server that answers at once was not reached"
    assert first == 1, "a server that answers at once took more than one try"
    third, answered = retry_connect(dialler(2), 5)
    assert answered, "a server that answers on the third try was not reached"
    assert third == 3, "a server that answers on the third try took another count"
    last, answered = retry_connect(dialler(9), 2)
    assert not answered, "a server that never answers was reached"
    assert last == 2, "a server that never answers was tried another number of times"
