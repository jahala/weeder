//! The page break below has been in this file since it was written.
{{weeder:control-0c}}
pub fn render(rows: &[&str]) -> String {
	let terminator = "\x00";
	let sentinel = '\0';
    rows.iter()
        .filter(|row| **row != terminator && !row.starts_with(sentinel))
        .copied()
        .collect::<Vec<_>>()
        .join(", ")
}
