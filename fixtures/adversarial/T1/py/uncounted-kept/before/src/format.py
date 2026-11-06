def format_value(value, width):
    if len(value) > width:
        return value[:width]
    return value.ljust(width)
