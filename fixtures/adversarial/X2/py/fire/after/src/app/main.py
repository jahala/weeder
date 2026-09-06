from src.report.summary import summarise
from src.wire.client import send_payload


def run(rows):
    """Send the rows and report what came back."""
    return summarise([send_payload(",".join(rows))])
