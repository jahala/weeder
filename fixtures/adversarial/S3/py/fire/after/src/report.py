import pdb


def summarise(rows):
    print("rows", rows)
    return ", ".join(rows)


def total(rows):
    breakpoint()
    pdb.set_trace()
    return sum(rows)
