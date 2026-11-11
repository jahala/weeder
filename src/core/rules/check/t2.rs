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
//! string are talk about a test rather than a test. A helper the repository
//! names under `[tests] assertions` is part of that vocabulary in every
//! language, because a suite that asserts through its own `t` makes its claims
//! there and nowhere else.
//!
//! An assertion is made once for every case it runs for. Three written out and
//! one inside a table of the same three cases are the same three claims, so a
//! line inside a parametrized case, or a loop over a table the file writes
//! down, counts once per entry, and a table that loses a row loses a claim.
//! Where the file does not write the cases down, a loop over them runs its
//! claims some number of times weeder cannot read, and a change that puts a
//! claim under such a loop is one whose fall weeder cannot prove. It is not
//! let through in silence, because deleting the claims and adding such a loop
//! would then pass unread; it is reported as a warning, which is for the
//! person at the pull request, and it says the loss is unproven rather than
//! claiming it. A table weeder can count still blocks when it loses a row.
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
//!
//! A claim can also be made stronger rather than kept. Three loose claims on
//! one value, that it contains this, that and the other, replaced by one claim
//! that the value equals the whole of what it should be, are not two claims
//! lost: the one says everything the three said. So a claim that left is not
//! counted where the other side compares the same value whole, in the same
//! case and with the same thing bound to it, or the value it was read off;
//! see [`subject`]. An exact claim on something else, the value's length,
//! covers nothing, and a claim that leaves with no exact one beside it is a
//! claim dropped.

use std::collections::BTreeSet;

use crate::core::change::{Change, Side};
use crate::core::finding::{Detection, Level, Message, Region, Stamp};
use crate::core::rules::check::generated::{nested_functions, Runs, Times};
use crate::core::rules::check::idiom::Block;
use crate::core::rules::check::subject::{self, Subject};
use crate::core::rules::check::vocab::Suite;
use crate::core::rules::check::Judgement;
use crate::core::syntax::Mask;

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let changes = judged.changes;
    // A file's claims are read once and asked two questions: what this file no
    // longer claims, and what every other file in the change claims now.
    let helpers = &judged.config.test_assertions;
    let read: Vec<Made> = changes.iter().map(|change| made(change, helpers)).collect();
    let arrived = arrivals(changes, &read);
    changes
        .iter()
        .zip(&read)
        .filter_map(|(change, made)| dropped(change, made, &arrived))
        .collect()
}

/// One line of a test file that makes claims, as a move can be followed by it:
/// the case it sits in, the line it is written on, and how many claims it
/// makes, which is its assertions times the cases a table runs them for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Claim {
    case: String,
    line: String,
    weight: usize,
    /// The values the claim is about, where its line can be read for them.
    subjects: Vec<Subject>,
}

/// A claim a file in this change did not make before it and makes now.
#[derive(Debug, Clone)]
struct Arrival {
    path: String,
    claim: Claim,
}

/// The claims one side of a file makes, and the ones among them a loop runs
/// over cases weeder cannot count. Those are counted once, which is the least
/// they can be, and they are kept apart so that a count resting on them is
/// never taken for the whole of what the file claims.
#[derive(Default)]
struct Said {
    made: Vec<Claim>,
    floors: Vec<Claim>,
    /// The values compared whole, each with the case that compares it.
    pinned: Vec<(String, Subject)>,
}

/// The claims one changed file makes on each side of the change.
struct Made {
    before: Said,
    after: Said,
}

/// Where the claims a file lost turned up, and how many of them did.
struct Followed {
    count: usize,
    into: BTreeSet<String>,
}

/// What the change did to this file's assertions, where it did anything.
fn dropped(change: &Change, made: &Made, arrived: &[Arrival]) -> Option<Detection> {
    let path = change.diff.new_path.as_deref()?;
    // A file the change took away took its assertions with it, and that is T1's
    // finding to make: one deletion, one finding.
    if change.is_deletion() || !change.before.holds_tests() || !change.after.holds_tests() {
        return None;
    }
    let before = total(&made.before.made);
    let after = total(&made.after.made);
    if after >= before {
        return None;
    }
    // A claim the change put under a loop whose cases weeder cannot count may
    // run as often as every claim that left, so the fall is one weeder cannot
    // prove, and a person has to read it. A loop that was there on both sides
    // takes nothing from the proof.
    let unproven = !unanswered(&made.after.floors, &made.before.floors).is_empty();
    // A claim the other side makes again, whole, says nothing less, and is
    // neither a claim dropped nor one to follow elsewhere. It is made again only
    // where every value it was about is compared whole again: a claim that held
    // one value to another is not said by a claim that holds a third to it.
    let (covered, lost): (Vec<Claim>, Vec<Claim>) = unanswered(&made.before.made, &made.after.made)
        .into_iter()
        .partition(|claim| {
            !claim.subjects.is_empty()
                && claim.subjects.iter().all(|value| {
                    made.after
                        .pinned
                        .iter()
                        .any(|(case, pinned)| *case == claim.case && pinned.covers(value))
                })
        });
    let moved = followed(&lost, arrived, path);
    let remaining = (before - after)
        .saturating_sub(moved.count)
        .saturating_sub(total(&covered));
    if remaining == 0 {
        return None;
    }
    let mut found = finding(path, change, before, after, remaining, &moved);
    if unproven {
        found.stamp = Stamp::Override(Level::Warn);
        found.message = uncounted(before, after, remaining, &moved);
    }
    Some(found)
}

/// What T2 says where the claims it cannot find may be running under a loop
/// over cases the file does not write down: the fall as written, and why only
/// a person can say whether anything was lost.
fn uncounted(before: usize, after: usize, remaining: usize, moved: &Followed) -> Message {
    let went = if moved.count == 0 {
        String::new()
    } else {
        format!(", {} moved to {}", counted(moved.count), named(&moved.into))
    };
    Message {
        what: format!(
            "{} went out of this file as written{went}: it made {before} and now writes {after}, and the rest now run under a loop over cases weeder cannot count.",
            counted(remaining)
        ),
        why: "the loop may run every claim that left or none of them, and only running it or reading it says which, so weeder cannot tell a suite that got shorter from one that stopped checking.".to_string(),
        next: "read the loop and the cases it runs over; if the claims are gone, put them back, and if they are there, write the cases down where weeder can count them.".to_string(),
    }
}

/// Both sides of one changed file, read for the claims they make, with the
/// helpers the repository names counted among them.
fn made(change: &Change, helpers: &[String]) -> Made {
    Made {
        before: claims(&change.before, helpers),
        after: claims(&change.after, helpers),
    }
}

/// How many claims a set of lines makes between them.
fn total(claims: &[Claim]) -> usize {
    claims
        .iter()
        .fold(0, |sum, claim| sum.saturating_add(claim.weight))
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
        for claim in unanswered(&made.after.made, &made.before.made) {
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
    let mut count: usize = 0;
    for (at, amount) in answered(&left, &landed).into_iter().flatten() {
        count = count.saturating_add(amount);
        into.insert(elsewhere[at].path.clone());
    }
    Followed { count, into }
}

/// The claims on the left that nothing on the right answers for, each carrying
/// the part of its weight left unanswered.
fn unanswered(left: &[Claim], right: &[Claim]) -> Vec<Claim> {
    let lost: Vec<&Claim> = left.iter().collect();
    let made: Vec<&Claim> = right.iter().collect();
    answered(&lost, &made)
        .into_iter()
        .zip(left)
        .filter_map(|(found, claim)| {
            let answered = found
                .iter()
                .fold(0_usize, |sum, (_, amount)| sum.saturating_add(*amount));
            let weight = claim.weight.saturating_sub(answered);
            (weight > 0).then(|| Claim {
                weight,
                ..claim.clone()
            })
        })
        .collect()
}

/// Each claim on the left with the claims on the right that answer for it, and
/// how much of it each one answers for, no claim on the right spoken for more
/// often than it is made. A claim's twin is paired first, the identical line in
/// the same case about the same values, then the identical line in the same
/// case, then the identical line anywhere, then a rewritten one, so no claim is
/// credited with a partner another claim has the better claim to. A suite that
/// writes one check in every case, or one check twice in a case about two
/// values, would otherwise pair a check that went with a copy that stayed, and
/// the claim read as lost would be one that is still there.
fn answered(left: &[&Claim], right: &[&Claim]) -> Vec<Vec<(usize, usize)>> {
    let mut found: Vec<Vec<(usize, usize)>> = vec![Vec::new(); left.len()];
    let mut owed: Vec<usize> = left.iter().map(|claim| claim.weight).collect();
    let mut free: Vec<usize> = right.iter().map(|claim| claim.weight).collect();
    for pass in [
        Pairing::Twin,
        Pairing::SameLineSameCase,
        Pairing::SameLine,
        Pairing::Answers,
    ] {
        for (index, claim) in left.iter().enumerate() {
            for (at, made) in right.iter().enumerate() {
                if owed[index] == 0 {
                    break;
                }
                let pairs = match pass {
                    Pairing::Twin => {
                        claim.line == made.line
                            && claim.case == made.case
                            && claim.subjects == made.subjects
                    }
                    Pairing::SameLineSameCase => claim.line == made.line && claim.case == made.case,
                    Pairing::SameLine => claim.line == made.line,
                    Pairing::Answers => answers(claim, made),
                };
                if free[at] == 0 || !pairs {
                    continue;
                }
                let amount = owed[index].min(free[at]);
                owed[index] -= amount;
                free[at] -= amount;
                found[index].push((at, amount));
            }
        }
    }
    found
}

/// The order claims are paired in, closest partner first.
#[derive(Clone, Copy)]
enum Pairing {
    Twin,
    SameLineSameCase,
    SameLine,
    Answers,
}

/// Whether a claim made elsewhere answers for one that left: the same line,
/// wherever it is now written, or the same case, however the line inside it was
/// rewritten on the way. A claim that sits in no case at all is followed by its
/// line alone, because a helper belongs to every case that calls it.
fn answers(left: &Claim, made: &Claim) -> bool {
    left.line == made.line || (!left.case.is_empty() && left.case == made.case)
}

/// The claims one side of a change makes, in the order the file makes them. A
/// line inside a table makes its claims once for every case the table runs it
/// for. A side that holds no tests makes none and is not read for any: every
/// changed file passes through here, and most of them are not tests.
fn claims(side: &Side, helpers: &[String]) -> Said {
    let mut said = Said::default();
    if !side.holds_tests() {
        return said;
    }
    let mask = side.mask();
    let suite = Suite::of(side.lang(), mask).counting(helpers);
    // The tables are read only once a line has made a claim, since most of the
    // files that pass through here make none.
    let mut runs: Option<(Runs, Vec<Block>)> = None;
    for line in 1..=mask.line_count() {
        let count = suite.assertions(mask, line);
        if count == 0 {
            continue;
        }
        let case = side.tests.case_at(line);
        let name = case.map_or("", |case| case.name.as_str());
        // A line that makes several claims is read for none of their subjects:
        // which value each one is about is not a question one line answers.
        let reading = (count == 1)
            .then(|| {
                subject::read(
                    side.lang(),
                    mask,
                    line,
                    case.map_or(1, |case| case.start_line),
                )
            })
            .flatten();
        if let Some(reading) = reading.as_ref().filter(|reading| reading.exact) {
            said.pinned.extend(
                reading
                    .subjects
                    .iter()
                    .map(|value| (name.to_string(), value.clone())),
            );
        }
        // A line weeder cannot read for its subject is a claim about nothing it
        // can name, and no subjects is exactly that: nothing covers it.
        let subjects = reading.map(|reading| reading.subjects).unwrap_or_default();
        let claim = |weight: usize| Claim {
            case: name.to_string(),
            line: written(mask, line),
            weight,
            subjects: subjects.clone(),
        };
        let (tables, nested) = runs.get_or_insert_with(|| {
            (
                Runs::of(side.lang(), mask),
                nested_functions(side.lang(), mask, &side.tests.units),
            )
        });
        // A claim in a function nested in the case runs as often as that
        // function is called, which nothing in the file counts.
        let called = nested
            .iter()
            .any(|block| block.first < line && line <= block.last);
        let times = if called {
            Times::Uncounted
        } else {
            tables.times(line)
        };
        match times {
            Times::Counted(0) => {}
            Times::Counted(times) => said.made.push(claim(count.saturating_mul(times))),
            Times::Uncounted => {
                said.made.push(claim(count));
                said.floors.push(claim(count));
            }
        }
    }
    said
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
) -> Detection {
    let message = if moved.count == 0 {
        Message {
            what: format!(
                "{} went out of this file: it made {before} and now makes {after}.",
                counted(remaining)
            ),
            why: "a case that checks nothing passes whatever the code does, so the suite reports green over behaviour held by nobody.".to_string(),
            next: "put the checks back, or carry a `Weeder-allow: T2` trailer saying which claim stopped being worth making.".to_string(),
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
            next: "make the rest again where they belong, or carry a `Weeder-allow: T2` trailer saying which claim stopped being worth making.".to_string(),
        }
    };
    Detection {
        stamp: Stamp::Rule,
        path: path.to_string(),
        region: change.first_edit().map(|line| Region {
            start_line: line,
            end_line: line,
        }),
        message,
        fix: None,
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
