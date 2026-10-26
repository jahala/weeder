pub fn format_value(value: &str, width: usize) -> String {
    let mut padded = value.to_string();
    while padded.len() < width {
        padded.push(' ');
    }
    padded
}
