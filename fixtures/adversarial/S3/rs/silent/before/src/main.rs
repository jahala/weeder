mod report;

fn main() {
    let rows = vec!["a".to_string(), "b".to_string()];
    let _ = report::summarise(&rows);
}
