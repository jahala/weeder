//! D1, a dependency manifest changed.
//!
//! A manifest and its lockfile decide what code the program is built from, which
//! is a decision somebody makes rather than one that arrives inside a change
//! about something else. So the change is reported wherever it lands, and it is
//! a note rather than a refusal, because adding a dependency is ordinary work.
//!
//! It stops being a note when the run named the paths the change was for and the
//! manifest is not one of them. Then the dependency was never part of what was
//! asked for, and weeder says so at the level that stops the change.

use crate::core::change::Change;
use crate::core::classify::FileKind;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::glob;
use crate::core::rules::check::Judgement;

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    judged
        .changes
        .iter()
        .filter_map(|change| manifest(change, judged.scope))
        .collect()
}

fn manifest(change: &Change, scope: &[String]) -> Option<Finding> {
    let path = change.path()?;
    if !change.after.is(FileKind::Manifest) && !change.before.is(FileKind::Manifest) {
        return None;
    }
    let allowed = glob::matches_any(scope, path);
    Some(Finding {
        rule: "D1".to_string(),
        level: if allowed { Level::Warn } else { Level::Block },
        path: path.to_string(),
        region: change.first_edit().map(|line| Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: "a dependency manifest changed.".to_string(),
            why: if allowed {
                "what the program is built from moved, and no test in the repository will notice."
                    .to_string()
            } else {
                "the run named the paths this change was for, and this manifest is not one of them."
                    .to_string()
            },
            next: "read the change to the manifest yourself, and land it on its own if it was not part of the work.".to_string(),
        },
        fix: None,
        suppressed: None,
    })
}
