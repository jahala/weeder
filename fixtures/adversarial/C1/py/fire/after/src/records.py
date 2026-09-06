import re


def parse(line):
    """Split a record into its fields."""
    return re.split(r"[,;]", line)
