def trim(value):
    return value.strip()


def slugify(value):
    return "-".join(trim(value).lower().split())
