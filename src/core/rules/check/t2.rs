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
//!
//! A claim can also leave a file without leaving the suite. Splitting one test
//! file into two, or folding two into one, takes claims out of one file and
//! makes them in another, and both files are in the same diff: nothing is
//! claimed less afterwards. So the claims a file no longer makes are looked for
//! in the rest of the change, by the case they sat in and by the line they are
//! written on, either of which follows a move the other cannot: a case that
//! went to another file takes a rewritten line with it, and a case renamed on
//! the way still writes the same line. Each answer is spoken for once, so two
//! claims are never followed by one. A claim found there is not a claim dropped
//! and is not counted as one, and what is left over is what the finding
//! reports. Only when nothing was found anywhere does weeder say the behaviour is
//! held by nobody; where some claims moved, the finding names the files they
//! moved into, because a sentence a reader can disprove in one click costs more
//! than the finding is worth.

use std::collections::BTreeSet;

use crate::core::change::{Change, Side};
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::read::TestUnit;
use crate::core::rules::check::vocab::Suite;
use crate::core::rules::check::Judgement;
use crate::core::syntax::Mask;

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let changes = judged.changes;
    // A file's claims are read once and asked two questions: what this file no
    // longer claims, and what every other file in the change claims now.
    let read: Vec<Made> = changes.iter().map(made).collect();
    let arrived = arrivals(changes, &read);
    changes
        .iter()
        .zip(&read)
        .filter_map(|(change, made)| dropped(change, made, &arrived))
        .collect()
}

/// One claim a test file makes, as a move can be followed by it: the case it
/// sits in, and the line it is written on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Claim {
    case: String,
    line: String,
}

/// A claim a file in this change did not make before it and makes now.
#[derive(Debug, Clone)]
struct Arrival {
    path: String,
    claim: Claim,
}

/// The claims one changed file makes on each side of the change.
struct Made {
    before: Vec<Claim>,
    after: Vec<Claim>,
}

/// Where the claims a file lost turned up, and how many of them did.
struct Followed {
    count: usize,
    into: BTreeSet<String>,
}

/// What the change did to this file's assertions, where it did anything.
fn dropped(change: &Change, made: &Made, arrived: &[Arrival]) -> Option<Finding> {
    let path = change.diff.new_path.as_deref()?;
    // A file the change took away took its assertions with it, and that is T1's
    // finding to make: one deletion, one finding.
    if change.is_deletion() || !change.before.holds_tests() || !change.after.holds_tests() {
        return None;
    }
    let before = made.before.len();
    let after = made.after.len();
    if after >= before {
        return None;
    }
    let moved = followed(&unanswered(&made.before, &made.after), arrived, path);
    let remaining = (before - after).saturating_sub(moved.count);
    if remaining == 0 {
        return None;
    }
    Some(finding(path, change, before, after, remaining, &moved))
}

/// Both sides of one changed file, read for the claims they make.
fn made(change: &Change) -> Made {
    Made {
        before: claims(&change.before),
        after: claims(&change.after),
    }
}

/// Every claim that arrived in a test file this change touched. A claim the
/// file already made is not an arrival, so a file left alone offers nowhere for
/// a dropped claim to have gone.
fn arrivals(changes: &[Change], read: &[Made]) -> Vec<Arrival> {
    let mut found = Vec::new();
    for (change, made) in changes.iter().zip(read) {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !change.after.holds_tests() {
            continue;
        }
        for claim in unanswered(&made.after, &made.before) {
            found.push(Arrival {
                path: path.to_string(),
                claim,
            });
        }
    }
    found
}

/// The claims a file lost that arrived somewhere else in the same change, and
/// the files they arrived in.
fn followed(lost: &[Claim], arrived: &[Arrival], from: &str) -> Followed {
    let elsewhere: Vec<&Arrival> = arrived
        .iter()
        .filter(|arrival| arrival.path != from)
        .collect();
    let landed: Vec<&Claim> = elsewhere.iter().map(|arrival| &arrival.claim).collect();
    let left: Vec<&Claim> = lost.iter().collect();
    let mut into = BTreeSet::new();
    let mut count = 0;
    for at in answered(&left, &landed).into_iter().flatten() {
        count += 1;
        into.insert(elsewhere[at].path.clone());
    }
    Followed { count, into }
}

/// The claims on the left that nothing on the right answers for.
fn unanswered(left: &[Claim], right: &[Claim]) -> Vec<Claim> {
    let lost: Vec<&Claim> = left.iter().collect();
    let made: Vec<&Claim> = right.iter().collect();
    answered(&lost, &made)
        .into_iter()
        .zip(left)
        .filter(|(found, _)| found.is_none())
        .map(|(_, claim)| claim.clone())
        .collect()
}

/// Each claim on the left with the one on the right that answers for it, where
/// one does, and each answer spoken for once. An identical line is paired
/// first, so a rewritten line is not credited with a partner the line it was
/// written as has the better claim to.
fn answered(left: &[&Claim], right: &[&Claim]) -> Vec<Option<usize>> {
    let mut found: Vec<Option<usize>> = vec![None; left.len()];
    let mut taken = vec![false; right.len()];
    for same_line in [true, false] {
        for (index, claim) in left.iter().enumerate() {
            if found[index].is_some() {
                continue;
            }
            let at = right.iter().enumerate().position(|(at, made)| {
                !taken[at]
                    && if same_line {
                        claim.line == made.line
                    } else {
                        answers(claim, made)
                    }
            });
            if let Some(at) = at {
                taken[at] = true;
                found[index] = Some(at);
            }
        }
    }
    found
}

/// Whether a claim made elsewhere answers for one that left: the same line,
/// wherever it is now written, or the same case, however the line inside it was
/// rewritten on the way. A claim that sits in no case at all is followed by its
/// line alone, because a helper belongs to every case that calls it.
fn answers(left: &Claim, made: &Claim) -> bool {
    left.line == made.line || (!left.case.is_empty() && left.case == made.case)
}

/// The claims one side of a change makes, in the order the file makes them. A
/// side that holds no tests makes none and is not read for any: every changed
/// file passes through here, and most of them are not tests.
fn claims(side: &Side) -> Vec<Claim> {
    if !side.holds_tests() {
        return Vec::new();
    }
    let mask = side.mask();
    let suite = Suite::of(side.lang(), mask);
    let cases: Vec<&TestUnit> = side.tests.cases().collect();
    let mut made = Vec::new();
    for line in 1..=mask.line_count() {
        let count = suite.assertions(mask, line);
        if count == 0 {
            continue;
        }
        let claim = Claim {
            case: enclosing(&cases, line).to_string(),
            line: written(mask, line),
        };
        made.extend(std::iter::repeat_n(claim, count));
    }
    made
}

/// The case a line sits in: the narrowest one that spans it, so a case written
/// inside another answers for its own lines. A line outside every case sits in
/// none.
fn enclosing<'a>(cases: &[&'a TestUnit], line: u32) -> &'a str {
    cases
        .iter()
        .filter(|case| case.start_line <= line && line <= case.end_line)
        .min_by_key(|case| case.end_line.saturating_sub(case.start_line))
        .map_or("", |case| case.name.as_str())
}

/// A line as the program writes it: the whole of it apart from the comments,
/// with the spacing taken out, so a claim re-indented or wrapped on its way
/// into another file still reads as the claim that left.
fn written(mask: &Mask, line: u32) -> String {
    mask.outside_comments(line)
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

fn finding(
    path: &str,
    change: &Change,
    before: usize,
    after: usize,
    remaining: usize,
    moved: &Followed,
) -> Finding {
    let message = if moved.count == 0 {
        Message {
            what: format!(
                "{} went out of this file: it made {before} and now makes {after}.",
                counted(remaining)
            ),
            why: "a case that checks nothing passes whatever the code does, so the suite reports green over behaviour held by nobody.".to_string(),
            next: "put the checks back, or carry a `Weed-allow: T2` trailer saying which claim stopped being worth making.".to_string(),
        }
    } else {
        Message {
            what: format!(
                "{} went out of this file and {} moved to {}: it made {before} and now makes {after}.",
                counted(remaining),
                counted(moved.count),
                named(&moved.into)
            ),
            why: "the claims that moved are still made where they landed; the ones left behind are made nowhere, so that much of the behaviour is unwatched.".to_string(),
            next: "make the rest again where they belong, or carry a `Weed-allow: T2` trailer saying which claim stopped being worth making.".to_string(),
        }
    };
    Finding {
        rule: "T2".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: change.first_edit().map(|line| Region {
            start_line: line,
            end_line: line,
        }),
        message,
        fix: None,
        suppressed: None,
    }
}

/// The files a set of claims moved into, in the order weeder sorts paths.
fn named(paths: &BTreeSet<String>) -> String {
    paths
        .iter()
        .map(|path| format!("`{path}`"))
        .collect::<Vec<String>>()
        .join(", ")
}

fn counted(assertions: usize) -> String {
    if assertions == 1 {
        "1 assertion".to_string()
    } else {
        format!("{assertions} assertions")
    }
}
