from src.contrast import contrast


def test_text_on_paper_reads_in_light():
    assert contrast("light", "text", "paper") >= 4.5


def test_muted_on_paper_reads_in_light():
    assert contrast("light", "muted", "paper") >= 4.5


def test_edge_on_paper_shows_in_light():
    assert contrast("light", "edge", "paper") >= 3


def test_focus_on_paper_shows_in_light():
    assert contrast("light", "focus", "paper") >= 3


def test_text_on_paper_reads_in_dark():
    assert contrast("dark", "text", "paper") >= 4.5


def test_muted_on_paper_reads_in_dark():
    assert contrast("dark", "muted", "paper") >= 4.5


def test_edge_on_paper_shows_in_dark():
    assert contrast("dark", "edge", "paper") >= 3


def test_focus_on_paper_shows_in_dark():
    assert contrast("dark", "focus", "paper") >= 3
