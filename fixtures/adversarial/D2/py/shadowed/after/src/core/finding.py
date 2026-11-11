from src.seams.git import read


def render(rule, path):
    return f"{rule} {path} {read(path)}"
