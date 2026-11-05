def label(name):
    return name.strip()


def translate(block):
    todo = block.get("todo")
    if todo != None:
        return todo
    unimplemented = [name for name in block if not block[name]]
    return label(", ".join(unimplemented))


def resume(name):
    raise NotImplementedError(name)
