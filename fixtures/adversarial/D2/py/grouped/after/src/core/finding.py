import src.faces.check, src.seams.git


def render(rule, path):
    return f"{rule} {path} {src.seams.git.read(path)} {src.faces.check.name(path)}"
