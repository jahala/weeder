from src.vectors import vector_name


def atlas(count):
    """Name every vector up to a count."""
    return [vector_name(index) for index in range(count)]
