# Split the record on the separator the header names.
TODO_LABEL = "FIXME: the queue shows this to whoever opens it"


def parse(line):
    """Split a record into its fields."""
    return line.split(",")


def todo_label():
    """What the queue shows."""
    return TODO_LABEL
