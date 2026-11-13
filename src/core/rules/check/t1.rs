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
//! the change, by the name their runner collects them under, still making at
//! least the assertions they made, and, where the move renamed them, by
//! everything they hold with that name taken out. A case found there is not a
//! deletion and is not counted as one, and what is left over is what the
//! finding reports. A case that moved with the function it tests is often
//! renamed and rewritten on the way, so neither is left to follow it by; it is
//! followed by the function instead, and only where the cases that arrived
//! assert at least as much as it did, see [`moved`]. Only when nothing was
//! found anywhere does weeder say the behaviour is covered by nobody; where
//! some cases moved, the finding names the files they moved into, because a
//! sentence a reader can disprove in one click costs more than the finding is
//! worth.
//!
//! Where the cases now come from a table whose entries the file does not write
//! down, a table another module exports or a function returns, the table may
//! run every case that left or none of them. A change that puts its cases
//! under such a table is one whose fall weeder cannot prove, so the finding is
//! a warning for the person at the pull request, saying the loss is unproven
//! rather than claiming it, the same answer T2 gives a claim moved under such a
//! loop. A table of that kind that was there on both sides takes nothing from
//! the proof, and a case gone beside it still blocks.

use std::collections::BTreeSet;

use crate::core::change::{Change, Side};
use crate::core::finding::{Detection, Level, Message, Region, Stamp};
use crate::core::read::TestUnit;
use crate::core::rules::check::generated;
use crate::core::rules::check::moved::{self, Ledger, Offer};
use crate::core::rules::check::t2;
use crate::core::rules::check::vocab::Suite;
use crate::core::rules::check::Judgement;
use crate::core::syntax::Mask;

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let changes = judged.changes;
    // A file's cases are read once and asked two questions: what this file no
    // longer has, and what every other file in the change has now.
    let read: Vec<Held> = changes.iter().map(held).collect();
    let arrived = arrivals(changes, &read);
    // An arrival that followed a case by its name or its body is that case,
    // and pays for no other.
    let mut used = vec![false; arrived.len()];
    let losses: Vec<Option<Loss>> = changes
        .iter()
        .zip(&read)
        .map(|(change, held)| deletion(change, held, &arrived, &mut used))
        .collect();
    // The cases nothing followed may have followed the function they called.
    // That is read only where a file still owes cases, since most changes
    // leave none stranded.
    let helpers = &judged.config.test_assertions;
    let mut trail: Option<(BTreeSet<String>, Ledger)> = None;
    let mut found = Vec::new();
    for (change, loss) in changes.iter().zip(losses) {
        let mut gone = match loss {
            None => continue,
            Some(Loss::Uncounted(path)) => {
                found.push(uncounted_file_gone(path));
                continue;
            }
            Some(Loss::Counted(gone)) => gone,
        };
        if !gone.stranded.is_empty() {
            let (kept, ledger) = trail.get_or_insert_with(|| {
                let kept = moved::kept(changes);
                let ledger = offers(changes, &arrived, &used, &kept, helpers);
                (kept, ledger)
            });
            gone.follow_calls(change, kept, ledger, helpers);
        }
        if gone.remaining > 0 {
            found.push(gone.finding(change));
        }
    }
    found
}

/// One case, as a move can be followed by it: the name its runner collects it
/// under, what it holds with that name taken out, so the same case under
/// another name reads the same, how many assertions it makes, and the case as
/// the reader gave it, so what it calls can be asked of it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Case {
    name: String,
    body: String,
    assertions: usize,
    unit: TestUnit,
}

/// A case a file in this change did not have before it and has now, and the
/// change it arrived in.
#[derive(Debug, Clone)]
struct Arrival {
    path: String,
    case: Case,
    change: usize,
}

/// What a change took out of one test file.
enum Loss<'a> {
    /// A test file deleted with no case weeder could count in it.
    Uncounted(&'a str),
    /// A file that runs fewer cases than it did.
    Counted(Gone<'a>),
}

/// A file that runs fewer cases than it did, before the cases nothing followed
/// are offered to the function they called.
struct Gone<'a> {
    path: &'a str,
    before: usize,
    after: usize,
    remaining: usize,
    moved: Followed,
    /// The cases that left and that nothing followed by name or body.
    stranded: Vec<Case>,
    /// Whether the file opens a table weeder cannot count that it did not
    /// open before.
    opened: bool,
}

impl Gone<'_> {
    /// Pays for each stranded case, whole, out of the assertions added in
    /// other test files to cases that call the function it called. A case
    /// paid for moved; one that is not is still gone.
    fn follow_calls(
        &mut self,
        change: &Change,
        kept: &BTreeSet<String>,
        ledger: &mut Ledger,
        helpers: &[String],
    ) {
        let weights = t2::weights(&change.before, helpers);
        for case in &self.stranded {
            if self.remaining == 0 {
                break;
            }
            let calls = moved::exercised(&change.before, &case.unit, kept);
            let owed = moved::weight(&change.before, &case.unit, &weights);
            if let Some(paid) = ledger.pay_whole(self.path, &calls, owed) {
                self.remaining -= 1;
                self.moved.count += 1;
                self.moved.into.extend(paid.from);
            }
        }
    }

    fn finding(&self, change: &Change) -> Detection {
        if change.is_deletion() {
            return file_gone(self.path, self.remaining, &self.moved);
        }
        let mut found = cases_gone(
            self.path,
            change,
            self.before,
            self.after,
            self.remaining,
            &self.moved,
        );
        if self.opened {
            found.stamp = Stamp::Override(Level::Warn);
            found.message = uncounted(self.before, self.after, self.remaining, &self.moved);
        }
        found
    }
}

/// What every test file the change touched offers to pay for cases that
/// followed their function: each case that arrived and followed no case by
/// its name or body, that calls one of the functions the change kept, with
/// the assertions it makes counted the way T2 counts them.
fn offers(
    changes: &[Change],
    arrived: &[Arrival],
    used: &[bool],
    kept: &BTreeSet<String>,
    helpers: &[String],
) -> Ledger {
    let calling: Vec<(&Arrival, BTreeSet<String>)> = arrived
        .iter()
        .zip(used)
        .filter(|(_, used)| !**used)
        .filter_map(|(arrival, _)| {
            let side = &changes.get(arrival.change)?.after;
            let calls = moved::exercised(side, &arrival.case.unit, kept);
            (!calls.is_empty()).then_some((arrival, calls))
        })
        .collect();
    // Each side is weighed once, however many of its cases offer.
    let mut weighed: Vec<(usize, Vec<(u32, usize)>)> = Vec::new();
    let mut offers = Vec::new();
    for (arrival, calls) in calling {
        let Some(change) = changes.get(arrival.change) else {
            continue;
        };
        let at = match weighed
            .iter()
            .position(|(index, _)| *index == arrival.change)
        {
            Some(at) => at,
            None => {
                weighed.push((arrival.change, t2::weights(&change.after, helpers)));
                weighed.len() - 1
            }
        };
        offers.push(Offer {
            path: arrival.path.clone(),
            calls,
            weight: moved::weight(&change.after, &arrival.case.unit, &weighed[at].1),
        });
    }
    Ledger::new(offers)
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
fn deletion<'a>(
    change: &'a Change,
    held: &Held,
    arrived: &[Arrival],
    used: &mut [bool],
) -> Option<Loss<'a>> {
    let path = change.path()?;
    if !change.before.holds_tests() {
        return None;
    }
    let census = generated::census(&change.before);
    let now = generated::census(&change.after);
    let (before, after) = (census.count, now.count);
    // A test file deleted with no case weeder could count in it, a bats suite
    // before its cases are read, or a file of helpers, still took whatever it
    // held out of the run. How much is not known, so a person reads it.
    if change.is_deletion() && before == 0 {
        return Some(Loss::Uncounted(path));
    }
    let gone = if change.is_deletion() {
        before
    } else if after < before {
        before - after
    } else {
        return None;
    };
    let (moved, stranded) = followed(&lost(held), arrived, path, used);
    let remaining = gone.saturating_sub(moved.count);
    (remaining > 0).then(|| {
        Loss::Counted(Gone {
            path,
            before,
            after,
            remaining,
            moved,
            stranded,
            opened: !opened_since(&now.uncounted, &census.uncounted).is_empty(),
        })
    })
}

/// The tables weeder cannot count that the file opens now and did not open
/// before, each spoken for once, so a file that opens the same one twice
/// opened a second.
fn opened_since(now: &[String], before: &[String]) -> Vec<String> {
    let mut left: Vec<&String> = before.iter().collect();
    now.iter()
        .filter(|table| match left.iter().position(|was| was == table) {
            Some(at) => {
                left.remove(at);
                false
            }
            None => true,
        })
        .cloned()
        .collect()
}

/// What T1 says where the cases it cannot find may come from a table whose
/// entries the file does not write down: the fall as written, and why only a
/// person can say whether a case was lost.
fn uncounted(before: usize, after: usize, remaining: usize, moved: &Followed) -> Message {
    let went = if moved.count == 0 {
        String::new()
    } else {
        format!(", {} moved to {}", counted(moved.count), named(&moved.into))
    };
    Message {
        what: format!(
            "{} disappeared from this file as written{went}: it declared {before} and now declares {after}, and the rest may now come from a table weeder cannot count.",
            counted(remaining)
        ),
        why: "the table may hold every case that left or none of them, and only running it or reading it says which, so weeder cannot tell a suite that moved its cases into a table from one that lost them.".to_string(),
        next: "read the table; if the cases are gone, put them back, and if they are there, write the table down where weeder can count it.".to_string(),
    }
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
    for (index, (change, held)) in changes.iter().zip(read).enumerate() {
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
                change: index,
            });
        }
    }
    found
}

/// The cases a file lost that arrived somewhere else in the same change, the
/// files they arrived in, and the cases that arrived nowhere. Each arrival
/// that followed a case is marked in `used`, indexed as `arrived` is.
fn followed(
    lost: &[Case],
    arrived: &[Arrival],
    from: &str,
    used: &mut [bool],
) -> (Followed, Vec<Case>) {
    let mut count = 0;
    let mut into = BTreeSet::new();
    let mut stranded = Vec::new();
    for case in lost {
        let landed: Vec<usize> = arrived
            .iter()
            .enumerate()
            .filter(|(_, arrival)| arrival.path != from)
            .filter(|(_, arrival)| covers(case, &arrival.case))
            .map(|(index, _)| index)
            .collect();
        if landed.is_empty() {
            stranded.push(case.clone());
            continue;
        }
        count += 1;
        for index in landed {
            used[index] = true;
            into.insert(arrived[index].path.clone());
        }
    }
    (Followed { count, into }, stranded)
}

/// Whether the second case is the first one again: the name the runner collects
/// it under, or, where the move renamed it, everything it holds apart from that
/// name. A case that holds nothing is followed by its name alone, because an
/// empty body is every empty body.
///
/// A name is only a label, and a label can be written over a case that checks
/// nothing, so a case followed by its name must still make at least the
/// assertions it made before. One that makes fewer is the old case deleted and
/// a new one written under its name.
fn covers(one: &Case, other: &Case) -> bool {
    let named = !one.name.is_empty() && one.name == other.name;
    (named && other.assertions >= one.assertions)
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
    let suite = Suite::of(side.lang(), mask);
    declared
        .into_iter()
        .map(|unit| written(mask, &suite, unit))
        .collect()
}

/// What a case holds: its lines without the comments and without the spacing,
/// and with its own name taken out, so a case that was renamed on its way into
/// another file still reads as the case that left. The name is taken out where
/// it is first written, which is the line that declares the case; a case that
/// says its own title again inside itself keeps that.
fn written(mask: &Mask, suite: &Suite, unit: &TestUnit) -> Case {
    let assertions = (unit.start_line..=unit.end_line)
        .map(|line| suite.assertions(mask, line))
        .sum();
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
        assertions,
        unit: unit.clone(),
    }
}

/// What T1 says where a test file went and weeder could count no case in it.
fn uncounted_file_gone(path: &str) -> Detection {
    Detection {
        stamp: Stamp::Override(Level::Warn),
        path: path.to_string(),
        region: None,
        message: Message {
            what: "a test file was deleted, and weeder could not count the cases it held.".to_string(),
            why: "a file of tests that is gone runs none of them, and only a person can say whether this one held tests or only helped them.".to_string(),
            next: "restore the file, move its tests where they still run, or write why it went in a `Weeder-allow: T1` trailer.".to_string(),
        },
        fix: None,
    }
}

fn file_gone(path: &str, cases: usize, moved: &Followed) -> Detection {
    let message = if moved.count == 0 {
        Message {
            what: format!("a test file was deleted, and {} went with it.", counted(cases)),
            why: "a suite that is gone reports nothing, so whatever it covered is now covered by nobody.".to_string(),
            next: "restore the file, move its cases into the file that takes its place, or write why it went in a `Weeder-allow: T1` trailer.".to_string(),
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
            next: "send the rest after the others, or say which behaviour stopped being worth a test in a `Weeder-allow: T1` trailer.".to_string(),
        }
    };
    Detection {
        stamp: Stamp::Rule,
        path: path.to_string(),
        region: None,
        message,
        fix: None,
    }
}

fn cases_gone(
    path: &str,
    change: &Change,
    before: usize,
    after: usize,
    remaining: usize,
    moved: &Followed,
) -> Detection {
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
    Detection {
        stamp: Stamp::Rule,
        path: path.to_string(),
        region: change.first_edit().map(|line| Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what,
            why: "a case that is gone cannot fail, so the behaviour it held the code to is now unwatched.".to_string(),
            next: "put the cases back, or say which behaviour stopped being worth a test in a `Weeder-allow: T1` trailer.".to_string(),
        },
        fix: None,
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
