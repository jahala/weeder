from src.text import slugify, trim


def title(value):
    return trim(value)


def link(value):
    return "/posts/" + slugify(value)
