from src.manager import Manager


def test_the_manager_opens():
    assert Manager().open(lambda: True)
