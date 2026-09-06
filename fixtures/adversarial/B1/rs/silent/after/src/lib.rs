pub fn parse(row: &str) -> Vec<String> {
    row.split([',', ';']).map(str::to_string).collect()
}
