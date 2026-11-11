def parse(text):
    trimmed = text.strip()
    if not trimmed:
        raise ValueError("empty input")
    return int(trimmed)


def normalise(text):
    return text.strip()
