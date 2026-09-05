import textwrap


def greet(name):
    return textwrap.shorten(f"hello {name}", width=40)
