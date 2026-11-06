//! T4, a tolerance or a timeout was widened.
//!
//! A test that used to hold a value to six decimals and now holds it to two has
//! not been fixed; it has been told to stop looking. The same goes for a wait
//! that grew until the flake went away.
//!
//! The finding is a pair: a line that went out and the line that came in to
//! replace it, in one rewrite. The two are read through the syntax mask and
//! compared with their numbers taken out, the same assertion, differently
//! numbered, so a rewritten line, a renamed value or a changed expectation is
//! not this rule's business. What makes a number a slack is the word it is
//! written under, which the vocabulary knows: a magnitude accepts more as it
//! grows, and a count of digits accepts more as it shrinks.
//!
//! The partner is found by that likeness rather than by position. A block that
//! wraps unchanged assertions moves every line of them one level deeper, and git
//! writes the move as the old lines out and the new ones in, the block's opening
//! line first, so the line in the same place is the assertion written above. A
//! line that comes back the same but for its whitespace only moved, and is set
//! aside before any partner is looked for.

use std::collections::{BTreeMap, VecDeque};

use crate::core::change::Run;
use crate::core::finding::{Detection, Message, Region, Stamp};
use crate::core::rules::check::vocab::{names, numbers, skeleton, slack, Name, Number, Sense};
use crate::core::rules::check::Judgement;
use crate::core::syntax::Mask;

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let changes = judged.changes;
    let mut findings = Vec::new();
    for change in changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !change.after.holds_tests() {
            continue;
        }
        let before = change.before.mask();
        let after = change.after.mask();
        for run in change.runs() {
            for (old_line, new_line) in partners(before, after, &run) {
                let out = before.code(old_line);
                let into = after.code(new_line);
                if let Some(widening) = widening(&out, &into) {
                    findings.push(finding(path, new_line, &widening));
                }
            }
        }
    }
    findings
}

/// The lines of one rewrite that are the same assertion differently numbered,
/// as the number each has in its own file.
///
/// A removed line that comes back the same once its whitespace is set aside is
/// that line moved, and it takes its copy out of the running first. Every other
/// removed line is set against the first added line still unclaimed that reads
/// the same with its numbers taken out, which is the only partner this rule
/// could judge. A line git could not number is not one a finding can point at,
/// and takes no part. The maps are looked up and never walked, so their order
/// reaches no output.
fn partners(before: &Mask, after: &Mask, run: &Run<'_>) -> Vec<(u32, u32)> {
    let run = run.rewritten();
    let rewritten: Vec<u32> = run
        .removed
        .iter()
        .filter_map(|line| line.old_line)
        .collect();
    let unclaimed: Vec<Option<u32>> = run.added.iter().map(|line| line.new_line).collect();
    let mut shapes: BTreeMap<String, VecDeque<u32>> = BTreeMap::new();
    for new_line in unclaimed.into_iter().flatten() {
        shapes
            .entry(skeleton(&after.code(new_line)))
            .or_default()
            .push_back(new_line);
    }
    rewritten
        .into_iter()
        .filter_map(|old_line| {
            let new_line = shapes
                .get_mut(&skeleton(&before.code(old_line)))?
                .pop_front()?;
            Some((old_line, new_line))
        })
        .collect()
}

/// What a rewritten line did to the slack it carries. The two lines read the
/// same with their numbers taken out, which is how they were paired, so the
/// numbers are the only thing that moved.
fn widening(out: &str, into: &str) -> Option<Widening> {
    let named = names(into);
    numbers(out)
        .into_iter()
        .zip(numbers(into))
        .find_map(|(was, now)| {
            let sense = widens(sense(&named, &now)?, was.value, now.value)?;
            Some(Widening {
                sense,
                was: number_text(out, &was),
                now: number_text(into, &now),
            })
        })
}

/// What a number is a slack of. The name that says so is the last one written
/// before it that says anything about slack at all, the keyword argument that
/// gave the number, the call that takes it, the function whose result it is
/// compared against, and where nothing before it says anything, the first name
/// after it, because a duration is as often written with its unit behind it.
fn sense(named: &[Name<'_>], number: &Number) -> Option<Sense> {
    let carries = |name: &Name<'_>| slack(name.last()).or_else(|| name.receiver().and_then(slack));
    named
        .iter()
        .rfind(|name| name.end <= number.start && carries(name).is_some())
        .or_else(|| {
            named
                .iter()
                .find(|name| name.start >= number.end && carries(name).is_some())
        })
        .and_then(carries)
}

/// Whether the number moved the way that accepts more, and what it turned out
/// to be a number of. A framework that spells nearness both ways is read off
/// the number itself: a slack is written with a fraction or an exponent, and a
/// count of digits is a whole number.
fn widens(sense: Sense, was: f64, now: f64) -> Option<Sense> {
    let settled = match sense {
        Sense::Nearness if was.fract() == 0.0 && now.fract() == 0.0 => Sense::Precision,
        Sense::Nearness => Sense::Magnitude,
        other => other,
    };
    let widened = match settled {
        Sense::Precision => now < was,
        _ => now > was,
    };
    widened.then_some(settled)
}

fn number_text(code: &str, number: &Number) -> String {
    code.get(number.start..number.end)
        .unwrap_or_default()
        .to_string()
}

/// A slack that grew, as it will be quoted back to the reader.
struct Widening {
    sense: Sense,
    was: String,
    now: String,
}

fn finding(path: &str, line: u32, widening: &Widening) -> Detection {
    let Widening { sense, was, now } = widening;
    let what = match sense {
        Sense::Precision => {
            format!("a test now holds a value to fewer digits: `{was}` became `{now}`.")
        }
        _ => format!("a test now allows more than it did: `{was}` became `{now}`."),
    };
    Detection {
        stamp: Stamp::Rule,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what,
            why: "the case that failed on this line passes now, and the behaviour that made it fail is still there.".to_string(),
            next: "put the number back, or say in the change what made the old one wrong.".to_string(),
        },
        fix: None,
    }
}
