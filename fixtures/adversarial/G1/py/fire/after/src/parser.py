def parse(text: str) -> list[str]:
{{weeder:ours}} HEAD
    return text.split(",")
{{weeder:separator}}
    return text.split(";")
{{weeder:theirs}} feature/split-on-semicolons
