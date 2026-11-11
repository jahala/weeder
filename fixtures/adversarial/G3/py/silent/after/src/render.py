# The page break below has been in this file since it was written.
{{weeder:control-0c}}
def render(rows: list[str]) -> str:
    terminator = "\x00"
    sentinel = '\0'
    kept = [row for row in rows if row not in (terminator, sentinel)]
    return ",	".join(kept)
