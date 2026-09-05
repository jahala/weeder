pub fn pad(text: &str, width: usize) -> String {
    format!("{text:>width$}")
}
