from src.client import label, send


def test_trims_the_payload():
    assert send(" a ") == "a"


def test_labels_work_still_to_do():
    # TODO: cover the retry path once the queue lands
    assert label("todo") == "TODO: written by the caller"
