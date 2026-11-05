from .errors import NotImplemented


def answer(code):
    return str(code)


def reply(error):
    if isinstance(error, NotImplemented):
        return "the peer has not written this method"
    return "failed"


def guarded(run):
    try:
        return run()
    except NotImplementedError as error:
        return reply(error)


def resume(name):
    raise NotImplementedError(name)
