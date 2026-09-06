pub fn parse_input(text: &str) -> Vec<String> {
    text.split(',').map(str::to_string).collect()
}
