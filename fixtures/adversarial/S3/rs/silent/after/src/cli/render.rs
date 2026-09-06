pub fn render(rows: &[String]) -> String {
    println!("{}", rows.join("\n"));
    rows.join("\n")
}
