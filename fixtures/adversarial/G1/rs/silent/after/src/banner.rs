const RULE: &str = "=======";

pub fn banner(title: &str) -> String {
    format!("{title}\n{RULE}")
}
