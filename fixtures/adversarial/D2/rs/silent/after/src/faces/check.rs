use crate::core::finding::{render, Finding};
use crate::seams::git::read;

pub fn check(path: &str) -> String {
    render(&Finding {
        rule: "T1".to_string(),
        path: read(path),
    })
}
