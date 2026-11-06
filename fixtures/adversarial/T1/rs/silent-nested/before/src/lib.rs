/// The ratio of `fg` over `bg` in a theme, or 1 where none is known.
pub fn contrast(theme: &str, fg: &str, bg: &str) -> f64 {
    match (theme, fg, bg) {
        ("light", "text", "paper") => 12.6,
        ("light", "muted", "paper") => 5.1,
        ("light", "edge", "paper") => 3.4,
        ("light", "focus", "paper") => 4.2,
        ("dark", "text", "paper") => 13.1,
        ("dark", "muted", "paper") => 5.6,
        ("dark", "edge", "paper") => 3.2,
        ("dark", "focus", "paper") => 4.8,
        _ => 1.0,
    }
}
