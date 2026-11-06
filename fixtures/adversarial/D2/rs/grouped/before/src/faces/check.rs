use crate::core::finding::{render, Finding};

pub fn check(path: &str) -> String {
    render(&Finding {
        rule: "T1".to_string(),
        path: path.to_string(),
    })
}
