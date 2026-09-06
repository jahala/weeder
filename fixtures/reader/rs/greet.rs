use std::fmt::Write;

pub fn greet(name: &str) -> String {
    let mut out = String::new();
    let _ = write!(out, "hello {name}");
    out
}

#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greets_by_name() {
        assert_eq!(greet("weeder"), "hello weeder");
    }
}
