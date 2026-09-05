from src.format import format_value


def test_pads_to_the_width():
    assert format_value("a", 3) == "a  "


# @pytest.mark.skip came off this case when the padding stopped rounding.
def test_truncates_past_the_width():
    assert format_value("abcd", 3) == "abc"


def test_names_the_markers_a_reviewer_greps_for():
    assert markers() == ["@pytest.mark.skip", "@pytest.mark.xfail", "@unittest.skip"]
