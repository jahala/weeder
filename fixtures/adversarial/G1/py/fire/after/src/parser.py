def parse(text: str) -> list[str]:
{{weed:ours}} HEAD
    return text.split(",")
{{weed:separator}}
    return text.split(";")
{{weed:theirs}} feature/split-on-semicolons
