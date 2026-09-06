pub fn parse(input: &str) -> Vec<&str> {
{{weeder:ours}} HEAD
    input.split(',').collect()
{{weeder:separator}}
    input.split(';').collect()
{{weeder:theirs}} feature/split-on-semicolons
}
