from greet import greet


class TestGreet:
    def test_greets_by_name(self):
        assert greet("weeder") == "hello weeder"


def test_greets_the_world():
    assert greet("world") == "hello world"
