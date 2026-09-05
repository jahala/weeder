pub fn parse(input: &str) -> Vec<&str> {
{{weed:ours}} HEAD
    input.split(',').collect()
{{weed:separator}}
    input.split(';').collect()
{{weed:theirs}} feature/split-on-semicolons
}
