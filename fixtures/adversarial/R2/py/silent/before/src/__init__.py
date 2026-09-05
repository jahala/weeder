from .greet import greet


def main():
    print(greet("world"))


def format_record(fields):
    return ",".join(fields)
