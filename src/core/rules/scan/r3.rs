//! R3 — a TODO is older than the configured age.
//!
//! A work marker is a promise with no date on it. Left alone it stops being a
//! plan and becomes a description of the code, and by then nobody remembers who
//! made it or whether it still holds. The age comes from git rather than from
//! anything written beside the marker: the line's own blame is the only record
//! that cannot be edited into agreeing with the comment.
//!
//! Only comments count. A work marker in a string is a message a program shows
//! somebody, and one in code is an identifier that happens to be spelled that
//! way; neither is a note left for the next reader.

use std::collections::BTreeSet;

use crate::core::config::Config;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::syntax;
use crate::core::tree::Tree;

/// The words a repository leaves work under.
const MARKERS: &[&str] = &["TODO", "FIXME", "XXX"];

/// Seconds in a day. git dates a line to the second; a threshold is in days.
const DAY: i64 = 86_400;

pub fn evaluate(tree: &Tree, config: &Config) -> Vec<Finding> {
    let allowed = i64::from(config.thresholds.todo_age_days);
    let mut findings = Vec::new();
    for (path, line, marker) in marked_lines(tree) {
        let Some(written) = tree
            .blame
            .get(&path)
            .and_then(|times| times.get(line as usize - 1))
        else {
            continue;
        };
        let days = (tree.now - written).max(0) / DAY;
        if days <= allowed {
            continue;
        }
        findings.push(finding(&path, line, &marker, days, allowed));
    }
    findings
}

/// The files carrying a work marker, so a face knows which ones are worth
/// asking git to date. Blaming a whole repository to age a handful of comments
/// would cost more than every other rule together.
#[must_use]
pub fn marked_files(tree: &Tree) -> BTreeSet<String> {
    marked_lines(tree)
        .into_iter()
        .map(|(path, _, _)| path)
        .collect()
}

/// Every work marker the tree's comments carry: the file, the 1-based line, and
/// the word that was written.
fn marked_lines(tree: &Tree) -> Vec<(String, u32, String)> {
    let mut found = Vec::new();
    for file in tree.code() {
        let mask = file.mask();
        for line in 1..=file.text().lines().count() as u32 {
            let comment = mask.comments(line);
            if comment.trim().is_empty() {
                continue;
            }
            for word in syntax::words(&comment) {
                if let Some(marker) = MARKERS.iter().find(|marker| **marker == word.text) {
                    found.push((file.path.clone(), line, (*marker).to_string()));
                    break;
                }
            }
        }
    }
    found
}

fn finding(path: &str, line: u32, marker: &str, days: i64, allowed: i64) -> Finding {
    Finding {
        rule: "R3".to_string(),
        level: Level::Warn,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!("this {marker} was written {days} days ago, and the repository allows {allowed}."),
            why: "a marker nobody has come back to has stopped being a plan and started describing the code, and the person who made the promise is gone.".to_string(),
            next: "do the work, write it down where work is tracked, or delete the marker.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
