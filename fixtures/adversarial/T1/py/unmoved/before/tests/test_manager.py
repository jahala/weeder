from src.manager import Manager, retry_connect


def dialler(failures):
    """A dial that answers after the given number of refusals."""
    calls = []

    def dial():
        calls.append(None)
        return len(calls) > failures

    return dial


def test_the_manager_opens():
    assert Manager().open(dialler(0))


def test_retry_connect_succeeds_at_once():
    attempts, connected = retry_connect(dialler(0), 3)
    assert connected is True
    assert attempts == 1


def test_retry_connect_retries_until_it_connects():
    attempts, connected = retry_connect(dialler(2), 5)
    assert connected is True
    assert attempts == 3


def test_retry_connect_gives_up():
    attempts, connected = retry_connect(dialler(9), 2)
    assert connected is False
    assert attempts == 2
