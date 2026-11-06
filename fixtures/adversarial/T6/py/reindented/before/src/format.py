class SeparatorError(Exception):
    pass


def parse(text):
    if not text:
        raise ValueError("input is empty")
    if ";;" in text:
        raise SeparatorError("stray separator")
    return text.split(";")
