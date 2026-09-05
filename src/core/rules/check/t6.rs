//! T6 — an error assertion was weakened.
//!
//! "It fails" is the weakest claim a test can make about a failure. The strong
//! ones name the kind and quote the message, and both are one edit away from
//! being dropped: the case still fails when the code is broken in some other
//! way, and passes when it is broken in the way that mattered.
//!
//! The finding is a pair, the line that went out against the line that came in.
//! Both have to be about a failure at all, and what they claim is read as a set
//! of qualifiers: the error kinds they name, and the messages they match on. A
//! pair that lost qualifiers and gained none was weakened. A pair that gained
//! any is a different claim — a kind swapped for another kind, a message
//! rewritten — and weed leaves it alone rather than guessing which is narrower.

use crate::core::change::{Change, Replacement};
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::rules::check::vocab::{failure_qualifiers, mentions_failure};
use crate::core::syntax::Mask;

pub fn evaluate(changes: &[Change]) -> Vec<Finding> {
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
        for pair in change.replacements() {
            if let Some(lost) = weakening(&before, &after, &pair) {
                findings.push(finding(path, pair.new_line, &lost));
            }
        }
    }
    findings
}

/// What a rewritten line stopped claiming about the failure it asserts on.
fn weakening(before: &Mask, after: &Mask, pair: &Replacement<'_>) -> Option<Vec<String>> {
    let out = before.code(pair.old_line);
    let into = after.code(pair.new_line);
    if !mentions_failure(&out) || !mentions_failure(&into) {
        return None;
    }
    let was = failure_qualifiers(before, pair.old_line);
    let now = failure_qualifiers(after, pair.new_line);
    // A pair that gained a qualifier made a different claim, not a weaker one.
    if now.difference(&was).next().is_some() {
        return None;
    }
    let lost: Vec<String> = was.difference(&now).cloned().collect();
    (!lost.is_empty()).then_some(lost)
}

fn finding(path: &str, line: u32, lost: &[String]) -> Finding {
    let named = lost
        .iter()
        .map(|qualifier| format!("`{qualifier}`"))
        .collect::<Vec<String>>()
        .join(", ");
    Finding {
        rule: "T6".to_string(),
        level: Level::Warn,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!("an assertion on a failure stopped naming {named}."),
            why: "a case that accepts any failure passes on the failure nobody meant, including the one the next change introduces.".to_string(),
            next: "name the error the code raises again, or say in the change why any failure will do.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
