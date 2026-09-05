//! T1, a test was deleted.
//!
//! Two shapes say the same thing. A test file removed takes every case it held
//! with it. A test file that stayed but declares fewer cases than it did lost
//! the difference. Both are counted from the reader's test shape at each side of
//! the change, never from the diff text: a case moved down the file, renamed in
//! place, or rewritten around still counts as one case, and only a case that
//! stopped existing changes the number.

use crate::core::change::Change;
use crate::core::finding::{Finding, Level, Message, Region};

pub fn evaluate(changes: &[Change]) -> Vec<Finding> {
    changes.iter().filter_map(deletion).collect()
}

/// What the change did to this file's tests, where it did anything at all.
fn deletion(change: &Change) -> Option<Finding> {
    let path = change.path()?;
    if !change.before.holds_tests() {
        return None;
    }
    let before = change.before.case_count();
    if change.is_deletion() {
        return Some(file_gone(path, before));
    }
    let after = change.after.case_count();
    if after >= before {
        return None;
    }
    Some(cases_gone(path, change, before, after))
}

fn file_gone(path: &str, cases: usize) -> Finding {
    Finding {
        rule: "T1".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: None,
        message: Message {
            what: format!("a test file was deleted, and {} went with it.", counted(cases)),
            why: "a suite that is gone reports nothing, so whatever it covered is now covered by nobody.".to_string(),
            next: "restore the file, or move its cases into the file that takes its place.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

fn cases_gone(path: &str, change: &Change, before: usize, after: usize) -> Finding {
    Finding {
        rule: "T1".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: first_edit(change).map(|line| Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!(
                "{} disappeared from this file: it declared {before} and now declares {after}.",
                counted(before - after)
            ),
            why: "a case that is gone cannot fail, so the behaviour it held the code to is now unwatched.".to_string(),
            next: "put the cases back, or say in the change which behaviour stopped being worth a test.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

/// Where in the new file the change first touched something, so the finding
/// points at the edit rather than at the top of a file nobody edited there.
fn first_edit(change: &Change) -> Option<u32> {
    let added = change.added().map(|(line, _)| line).min();
    let hunks = change
        .diff
        .hunks
        .iter()
        .map(|hunk| hunk.new_start.max(1))
        .min();
    added.or(hunks)
}

fn counted(cases: usize) -> String {
    if cases == 1 {
        "1 test case".to_string()
    } else {
        format!("{cases} test cases")
    }
}
