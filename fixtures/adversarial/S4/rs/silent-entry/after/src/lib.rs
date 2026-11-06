pub fn trim(value: &str) -> &str {
    value.trim()
}

pub fn slugify(value: &str) -> String {
    trim(value)
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}
