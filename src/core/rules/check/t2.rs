//! T2, assertions were dropped from a changed test file.
//!
//! A case that is still there and no longer checks anything is a test in name
//! only: it runs, it passes, and it holds the code to nothing. T1 counts the
//! cases; this one counts what they claim.
//!
//! The count is taken from each side of the change whole, never from the diff
//! text, so an assertion moved between cases, rewritten around, or re-indented
//! still counts as one. What makes an assertion is the language's own
//! vocabulary, read off the code alone: the same words in a comment or inside a
//! string are talk about a test rather than a test.

use crate::core::change::Change;
use crate::core::classify::Lang;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::rules::check::vocab::Suite;

pub fn evaluate(changes: &[Change]) -> Vec<Finding> {
    changes.iter().filter_map(dropped).collect()
}

/// What the change did to this file's assertions, where it did anything.
fn dropped(change: &Change) -> Option<Finding> {
    let path = change.diff.new_path.as_deref()?;
    // A file the change took away took its assertions with it, and that is T1's
    // finding to make: one deletion, one finding.
    if change.is_deletion() || !change.before.holds_tests() || !change.after.holds_tests() {
        return None;
    }
    let before = count(change, Side::Before);
    let after = count(change, Side::After);
    if after >= before {
        return None;
    }
    Some(finding(path, change, before, after))
}

enum Side {
    Before,
    After,
}

/// How many assertions one side of the change makes.
fn count(change: &Change, side: Side) -> usize {
    let side = match side {
        Side::Before => &change.before,
        Side::After => &change.after,
    };
    let lang = side
        .classification
        .as_ref()
        .map_or(Lang::Other, |classification| classification.lang);
    let mask = side.mask();
    Suite::of(lang, &mask).assertion_count(&mask)
}

fn finding(path: &str, change: &Change, before: usize, after: usize) -> Finding {
    Finding {
        rule: "T2".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: change.first_edit().map(|line| Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!(
                "{} went out of this file: it made {before} and now makes {after}.",
                counted(before - after)
            ),
            why: "a case that checks nothing passes whatever the code does, so the suite reports green over behaviour nobody is holding.".to_string(),
            next: "put the checks back, or carry a `Weed-allow: T2` trailer saying which claim stopped being worth making.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

fn counted(assertions: usize) -> String {
    if assertions == 1 {
        "1 assertion".to_string()
    } else {
        format!("{assertions} assertions")
    }
}
