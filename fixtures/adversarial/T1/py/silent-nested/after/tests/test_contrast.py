import pytest

from src.contrast import contrast

THEMES = ["light", "dark"]

TEXT_PAIRS = [
    ("text", "paper"),
    ("muted", "paper"),
]

EDGES = ["edge"]

MARKS = [*EDGES, "focus"]


@pytest.mark.parametrize("theme", THEMES)
class TestContrast:
    @pytest.mark.parametrize(("fg", "bg"), TEXT_PAIRS)
    def test_text_reads(self, theme, fg, bg):
        assert contrast(theme, fg, bg) >= 4.5

    @pytest.mark.parametrize("ground", ["paper"])
    @pytest.mark.parametrize("mark", MARKS)
    def test_mark_shows(self, theme, mark, ground):
        assert contrast(theme, mark, ground) >= 3
