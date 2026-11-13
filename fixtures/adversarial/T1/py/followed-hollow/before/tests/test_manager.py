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
    retry_connect(dialler(0), 3)


def test_retry_connect_retries_until_it_connects():
    retry_connect(dialler(2), 5)


def test_retry_connect_gives_up():
    retry_connect(dialler(9), 2)
