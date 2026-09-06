import re


def parse(row):
    """Split a row of the supplier feed into its fields."""
    return re.split(r"[,;]", row)
