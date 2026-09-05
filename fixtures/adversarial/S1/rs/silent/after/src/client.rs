pub fn send(payload: &str) -> String {
    payload.trim().to_string()
}

pub fn label(kind: &str) -> &'static str {
    if kind == "todo" {
        "TODO: written by the caller"
    } else {
        "done"
    }
}

pub fn version() -> &'static str {
    "1.4.0"
}
