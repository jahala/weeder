pub fn format_value(value: &str, width: usize) -> String {
    if value.len() > width {
        value[..width].to_string()
    } else {
        format!("{value:width$}")
    }
}

pub fn parse(input: &str) -> Vec<&str> {
    if input.is_empty() {
        Vec::new()
    } else {
        input.split(',').collect()
    }
}
