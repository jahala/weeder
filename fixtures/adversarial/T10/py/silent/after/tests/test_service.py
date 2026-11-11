from src.service import SERVICE, endpoint


def test_endpoint_builds_a_url_from_the_host():
    assert endpoint("example.org") == "https://example.org/"


def test_endpoint_points_at_the_build_service():
    assert endpoint(SERVICE) == "https://build.example.app/"
