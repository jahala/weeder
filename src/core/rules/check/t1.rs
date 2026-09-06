//! T1, a test was deleted.
//!
//! Two shapes say the same thing. A test file removed takes every case it held
//! with it. A test file that stayed but declares fewer cases than it did lost
//! the difference. Both are counted from the reader's test shape at each side of
//! the change, never from the diff text: a case moved down the file, renamed in
//! place, or rewritten around still counts as one case, and only a case that
//! stopped existing changes the number.
//!
//! The count is of the cases a file runs, not of the ones it writes out. A
//! suite that moves its cases into a table runs every one of them still, so
//! where a declaration is driven by a table the entries are what counts; see
//! [`generated`]. A table that leaves a case behind is a case gone like any
//! other.
//!
//! A case can also leave a file without leaving the suite. Splitting one file
//! into two, or folding two into one, takes cases out of one file and puts them
//! into another, and both files are in the same diff: nothing is covered less
//! afterwards. So the cases a file no longer has are looked for in the rest of
//! the change, by the name their runner collects them under and, where the move
//! renamed them, by everything they hold with that name taken out. A case found
//! there is not a deletion and is not counted as one, and what is left over is
//! what the finding reports. Only when nothing was found anywhere does weeder say
//! the behaviour is covered by nobody; where some cases moved, the finding names
//! the files they moved into, because a sentence a reader can disprove in one
//! click costs more than the finding is worth.

use std::collections::BTreeSet;

use crate::core::change::{Change, Side};
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::read::TestUnit;
use crate::core::rules::check::generated;
use crate::core::rules::check::Judgement;
use crate::core::syntax::Mask;

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let changes = judged.changes;
    // A file's cases are read once and asked two questions: what this file no
    // longer has, and what every other file in the change has now.
    let read: Vec<Held> = changes.iter().map(held).collect();
    let arrived = arrivals(changes, &read);
    changes
        .iter()
        .zip(&read)
        .filter_map(|(change, held)| deletion(change, held, &arrived))
        .collect()
}

/// One case, as a move can be followed by it: the name its runner collects it
/// under, and what it holds with that name taken out, so the same case under
/// another name reads the same.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Case {
    name: String,
    body: String,
}

/// A case a file in this change did not have before it and has now.
#[derive(Debug, Clone)]
struct Arrival {
    path: String,
    case: Case,
}

/// The cases one changed file holds on each side of the change.
struct Held {
    before: Vec<Case>,
    after: Vec<Case>,
}

/// Where the cases a file lost turned up, and how many of them did.
struct Followed {
    count: usize,
    into: BTreeSet<String>,
}

/// What the change did to this file's tests, where it did anything at all.
fn deletion(change: &Change, held: &Held, arrived: &[Arrival]) -> Option<Finding> {
    let path = change.path()?;
    if !change.before.holds_tests() {
        return None;
    }
    let before = generated::case_count(&change.before);
    let after = generated::case_count(&change.after);
    let gone = if change.is_deletion() {
        before
    } else if after < before {
        before - after
    } else {
        return None;
    };
    let moved = followed(&lost(held), arrived, path);
    let remaining = gone.saturating_sub(moved.count);
    if remaining == 0 {
        return None;
    }
    Some(if change.is_deletion() {
        file_gone(path, remaining, &moved)
    } else {
        cases_gone(path, change, before, after, remaining, &moved)
    })
}

/// Both sides of one changed file, read for the cases they declare.
fn held(change: &Change) -> Held {
    Held {
        before: cases(&change.before),
        after: cases(&change.after),
    }
}

/// The cases a file had and no longer has, under any name.
fn lost(held: &Held) -> Vec<Case> {
    held.before
        .iter()
        .filter(|case| !held.after.iter().any(|now| covers(case, now)))
        .cloned()
        .collect()
}

/// Every case that arrived in a test file this change touched. A case the file
/// already had is not an arrival, so a file left alone offers nowhere for a
/// deleted case to have gone.
fn arrivals(changes: &[Change], read: &[Held]) -> Vec<Arrival> {
    let mut found = Vec::new();
    for (change, held) in changes.iter().zip(read) {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !change.after.holds_tests() {
            continue;
        }
        for case in &held.after {
            if held.before.iter().any(|was| covers(was, case)) {
                continue;
            }
            found.push(Arrival {
                path: path.to_string(),
                case: case.clone(),
            });
        }
    }
    found
}

/// The cases a file lost that arrived somewhere else in the same change, and
/// the files they arrived in.
fn followed(lost: &[Case], arrived: &[Arrival], from: &str) -> Followed {
    let mut count = 0;
    let mut into = BTreeSet::new();
    for case in lost {
        let landed: Vec<&Arrival> = arrived
            .iter()
            .filter(|arrival| arrival.path != from)
            .filter(|arrival| covers(case, &arrival.case))
            .collect();
        if landed.is_empty() {
            continue;
        }
        count += 1;
        into.extend(landed.into_iter().map(|arrival| arrival.path.clone()));
    }
    Followed { count, into }
}

/// Whether the second case is the first one again: the name the runner collects
/// it under, or, where the move renamed it, everything it holds apart from that
/// name. A case that holds nothing is followed by its name alone, because an
/// empty body is every empty body.
fn covers(one: &Case, other: &Case) -> bool {
    (!one.name.is_empty() && one.name == other.name)
        || (!one.body.is_empty() && one.body == other.body)
}

/// The cases one side of a change declares, each as a move can be followed by
/// it. A side that declares none is not read at all: every changed file passes
/// through here, and most of them are not tests.
fn cases(side: &Side) -> Vec<Case> {
    let declared: Vec<&TestUnit> = side.tests.cases().collect();
    if declared.is_empty() {
        return Vec::new();
    }
    let mask = side.mask();
    declared
        .into_iter()
        .map(|unit| written(mask, unit))
        .collect()
}

/// What a case holds: its lines without the comments and without the spacing,
/// and with its own name taken out, so a case that was renamed on its way into
/// another file still reads as the case that left. The name is taken out where
/// it is first written, which is the line that declares the case; a case that
/// says its own title again inside itself keeps that.
fn written(mask: &Mask, unit: &TestUnit) -> Case {
    let text = (unit.start_line..=unit.end_line)
        .map(|line| mask.outside_comments(line))
        .map(|line| line.split_whitespace().collect::<Vec<&str>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<String>>()
        .join("\n");
    let body = if unit.name.is_empty() {
        text
    } else {
        text.replacen(&unit.name, "", 1)
    };
    Case {
        name: unit.name.clone(),
        body,
    }
}

fn file_gone(path: &str, cases: usize, moved: &Followed) -> Finding {
    let message = if moved.count == 0 {
        Message {
            what: format!("a test file was deleted, and {} went with it.", counted(cases)),
            why: "a suite that is gone reports nothing, so whatever it covered is now covered by nobody.".to_string(),
            next: "restore the file, or move its cases into the file that takes its place.".to_string(),
        }
    } else {
        Message {
            what: format!(
                "a test file was deleted: {} moved to {}, and {} went with it.",
                counted(moved.count),
                named(&moved.into),
                counted(cases)
            ),
            why: "the cases that moved run where they landed; the ones left behind cannot fail any more, so the behaviour they held the code to is unwatched.".to_string(),
            next: "send the rest after the others, or say in the change which behaviour stopped being worth a test.".to_string(),
        }
    };
    Finding {
        rule: "T1".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: None,
        message,
        fix: None,
        suppressed: None,
    }
}

fn cases_gone(
    path: &str,
    change: &Change,
    before: usize,
    after: usize,
    remaining: usize,
    moved: &Followed,
) -> Finding {
    let what = if moved.count == 0 {
        format!(
            "{} disappeared from this file: it declared {before} and now declares {after}.",
            counted(remaining)
        )
    } else {
        format!(
            "{} disappeared from this file and {} moved to {}: it declared {before} and now declares {after}.",
            counted(remaining),
            counted(moved.count),
            named(&moved.into)
        )
    };
    Finding {
        rule: "T1".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: change.first_edit().map(|line| Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what,
            why: "a case that is gone cannot fail, so the behaviour it held the code to is now unwatched.".to_string(),
            next: "put the cases back, or say in the change which behaviour stopped being worth a test.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

/// The files a set of cases moved into, in the order weeder sorts paths.
fn named(paths: &BTreeSet<String>) -> String {
    paths
        .iter()
        .map(|path| format!("`{path}`"))
        .collect::<Vec<String>>()
        .join(", ")
}

fn counted(cases: usize) -> String {
    if cases == 1 {
        "1 test case".to_string()
    } else {
        format!("{cases} test cases")
    }
}
