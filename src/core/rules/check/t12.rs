//! T12, an added case passes a literal in and checks that it came back out.
//!
//! A case that builds `new Error("Workspace not found")`, hands it to the code,
//! and checks that the answer contains `Workspace not found` holds the code to
//! one thing: that the string gets through. The code can drop the markup, the
//! wording, the condition the message was for, and the case stays green.
//!
//! weeder reports a case the change added where every check holds what it
//! reads to a string the case already wrote on an earlier line that is not a
//! check: the value going in, a prop, a fake's message. Only the strings the
//! check holds its subject to count, never one inside the subject itself,
//! which is how the value is looked up rather than what it is expected to be.
//! A string of nothing but punctuation or space says nothing and is passed
//! over. The neighbour checks the whole answer against a string of its own, the
//! message inside the markup the code adds, which the code can get wrong.
//!
//! A case that stores a value and reads it back has the same shape and can be
//! honest about a store, which is why this warns: the person at the pull
//! request can tell which of the two they are looking at.

use crate::core::finding::{Detection, Message, Region, Stamp};
use crate::core::rules::check::added::{self, Added, Check};
use crate::core::rules::check::Judgement;

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    added::cases(judged.changes)
        .iter()
        .filter_map(|added| {
            let inputs = inputs(added);
            let echoed: Vec<(u32, String)> = added
                .checks
                .iter()
                .map(|check| echo(check, &inputs))
                .collect::<Option<Vec<(u32, String)>>>()?;
            let (line, literal) = echoed.first()?;
            Some(finding(added, *line, literal))
        })
        .collect()
}

/// Every string the case writes on a line of its own that is not a check,
/// with the line it writes it on.
fn inputs(added: &Added) -> Vec<(u32, String)> {
    let mask = added.side.mask();
    added
        .lines()
        .into_iter()
        .filter(|line| !added.checks_on(*line))
        .flat_map(|line| {
            mask.literal_runs(line)
                .into_iter()
                .map(move |run| (line, run.text.trim_matches(['"', '\'', '`']).to_string()))
        })
        .filter(|(_, text)| says_something(text))
        .collect()
}

/// The line a check compares on and the string it reads back, where it holds
/// its subject to a string the case wrote going in.
fn echo(check: &Check, inputs: &[(u32, String)]) -> Option<(u32, String)> {
    let reading = check.reading.as_ref()?;
    reading
        .literals
        .iter()
        .find(|literal| {
            says_something(literal)
                && inputs
                    .iter()
                    .any(|(line, input)| *line < reading.line && input == *literal)
        })
        .map(|literal| (reading.line, literal.clone()))
}

/// Whether a string carries a letter or a digit, the least a message needs.
fn says_something(text: &str) -> bool {
    text.chars().any(char::is_alphanumeric)
}

fn finding(added: &Added, line: u32, literal: &str) -> Detection {
    Detection {
        stamp: Stamp::Rule,
        path: added.path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!(
                "the added case `{}` passes \"{literal}\" in and checks only that the same string comes back out.",
                added.case.name
            ),
            why: "a string the code only has to pass along holds the code to nothing it does, so the case stays green while the behaviour around the string breaks.".to_string(),
            next: "check what the code makes of the input, the whole answer or a value it computes, rather than the string it was handed.".to_string(),
        },
        fix: None,
    }
}
