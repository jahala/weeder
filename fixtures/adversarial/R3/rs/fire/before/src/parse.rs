// TODO: split the record on the separator the header names
pub fn parse(line: &str) -> Vec<String> {
    line.split(',').map(str::to_string).collect()
}
