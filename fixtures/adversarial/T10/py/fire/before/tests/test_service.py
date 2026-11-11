from src.service import endpoint


def test_endpoint_builds_a_url_from_the_host():
    assert endpoint("example.org") == "https://example.org/"
