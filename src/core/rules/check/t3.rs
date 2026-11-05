//! T3, a skip or a focus marker was added.
//!
//! Every runner weeder reads has two ways to stop a suite reporting: turn a case
//! off, or turn every other case off. Both leave a green run behind, which is
//! why the marker itself is the finding.
//!
//! The markers are read off the code and nothing else. A line is scanned through
//! the syntax mask first, so the same characters inside a string or a comment ,
//! a case titled "skip the empty input", a note explaining why a marker was
//! taken out, say nothing. Each language spells its markers in its own grammar:
//! a member on a declarator, a decorator, an attribute, a method on the handle
//! the runner passes in.

use crate::core::change::Change;
use crate::core::classify::Lang;
use crate::core::finding::{Detection, Fix, Message, Region, Stamp};
use crate::core::rules::check::Judgement;
use crate::core::syntax::{words, Mask};

/// The functions that declare a test where the language spells one as a call.
/// A marker is one of these with a member on it, or one of them wearing the
/// prefix its runner reads as "off" or "only this one".
const DECLARATORS: &[&str] = &["it", "test", "describe", "context", "suite", "bench"];

/// The members that turn a declarator into a marker, and what each one does.
const MEMBERS: &[(&str, Effect)] = &[
    ("skip", Effect::Skipped),
    ("todo", Effect::Skipped),
    ("failing", Effect::Skipped),
    ("only", Effect::Focused),
];

/// The letters a runner puts in front of a declarator instead of a member.
const PREFIXES: &[(char, Effect)] = &[('x', Effect::Skipped), ('f', Effect::Focused)];

/// The last segment of a decorator that takes a case out of the run.
const DECORATORS: &[&str] = &["skip", "skipif", "skipunless", "xfail"];

/// The attribute that leaves a test compiled and unrun.
const ATTRIBUTE: &str = "ignore";

/// The method on the testing handle that abandons the case it is called in.
const HANDLE_METHOD: &str = "Skip";

/// What a marker does to the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Effect {
    Skipped,
    Focused,
}

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
        let lang = language(change);
        let mask = change.after.mask();
        for (line, _) in change.added() {
            if let Some(marker) = marker(lang, mask, line) {
                findings.push(finding(path, line, &marker));
            }
        }
    }
    findings
}

/// The marker an added line carries, read from the code alone.
fn marker(lang: Lang, mask: &Mask, line: u32) -> Option<Marker> {
    let code = mask.code(line);
    match lang {
        Lang::TypeScript | Lang::JavaScript => {
            called_marker(&code, !mask.literals(line).trim().is_empty())
        }
        Lang::Python => decorator_marker(&code),
        Lang::Rust => attribute_marker(&code),
        Lang::Go => handle_marker(&code),
        Lang::Other => None,
    }
}

/// A declarator with a marker on it, `it.skip(`, `describe.only(`, or one
/// written with the prefix that does the same thing, `xit(`, `fdescribe(`.
///
/// The prefixed form is a declarator, so it is only read as one where the line
/// also carries the title a declarator is given. Without that, `fit(points)` is
/// a function with an unlucky name.
fn called_marker(code: &str, titled: bool) -> Option<Marker> {
    for word in words(code) {
        if word.is_member() && word.is_called() {
            if let Some((_, effect)) = MEMBERS.iter().find(|(name, _)| *name == word.text) {
                return Some(Marker::new(format!(".{}", word.text), *effect));
            }
        }
        if !titled || !word.is_called() || word.is_member() {
            continue;
        }
        let mut letters = word.text.chars();
        let Some(first) = letters.next() else {
            continue;
        };
        let rest: String = letters.collect();
        if !DECLARATORS.contains(&rest.as_str()) {
            continue;
        }
        if let Some((_, effect)) = PREFIXES.iter().find(|(letter, _)| *letter == first) {
            return Some(Marker::new(word.text.to_string(), *effect));
        }
    }
    None
}

/// The declarators that declare a case rather than a suite.
const CASE_DECLARATORS: &[&str] = &["it", "test", "specify"];

/// Whether a line of TypeScript or JavaScript declares a case with a marker on
/// it, `it.skip(`, `test.todo(`, `xit(`. The reader counts a case by the bare
/// declarator alone, so a case turned off reads to it as a case gone; T1 asks
/// this to count it as the case it still is (#74), and the marker stays this
/// rule's finding.
#[must_use]
pub fn declares_a_marked_case(code: &str, titled: bool) -> bool {
    let spoken = words(code);
    spoken
        .iter()
        .zip(spoken.iter().skip(1))
        .any(|(first, second)| {
            CASE_DECLARATORS.contains(&first.text)
                && !first.is_member()
                && second.is_member()
                && second.is_called()
                && MEMBERS.iter().any(|(name, _)| *name == second.text)
        })
        || (titled
            && spoken.iter().any(|word| {
                word.is_called()
                    && !word.is_member()
                    && word.text.len() > 1
                    && PREFIXES
                        .iter()
                        .any(|(letter, _)| word.text.starts_with(*letter))
                    && CASE_DECLARATORS.contains(&&word.text[1..])
            }))
}

/// A decorator that takes the case out of the run, whichever module it came
/// from: the last segment of the name is what decides.
fn decorator_marker(code: &str) -> Option<Marker> {
    let name = code.trim().strip_prefix('@')?;
    let path = name
        .split(|character: char| character == '(' || character.is_whitespace())
        .next()
        .unwrap_or(name);
    let last = path.rsplit('.').next().unwrap_or(path).to_ascii_lowercase();
    DECORATORS
        .contains(&last.as_str())
        .then(|| Marker::new(format!("@{path}"), Effect::Skipped))
}

/// The attribute that compiles a test and does not run it.
fn attribute_marker(code: &str) -> Option<Marker> {
    let inside = code.trim().strip_prefix("#[")?;
    let path = inside
        .split(|character: char| {
            matches!(character, ']' | '(' | '=' | ',') || character.is_whitespace()
        })
        .next()
        .unwrap_or(inside);
    (path.rsplit("::").next().unwrap_or(path) == ATTRIBUTE)
        .then(|| Marker::new(format!("#[{ATTRIBUTE}]"), Effect::Skipped))
}

/// A call to the testing handle's own way of abandoning a case.
fn handle_marker(code: &str) -> Option<Marker> {
    words(code).into_iter().find_map(|word| {
        (word.is_member() && word.is_called() && word.text.starts_with(HANDLE_METHOD))
            .then(|| Marker::new(format!(".{}", word.text), Effect::Skipped))
    })
}

/// A marker as it will be quoted back to the reader.
struct Marker {
    written: String,
    effect: Effect,
}

impl Marker {
    fn new(written: String, effect: Effect) -> Marker {
        Marker { written, effect }
    }
}

fn language(change: &Change) -> Lang {
    change
        .after
        .classification
        .as_ref()
        .map_or(Lang::Other, |classification| classification.lang)
}

fn finding(path: &str, line: u32, marker: &Marker) -> Detection {
    let (what, why) = match marker.effect {
        Effect::Skipped => (
            format!("a skip marker was added to a test: `{}`.", marker.written),
            "a skipped case reports as a pass, so the suite stays green over code nobody ran.",
        ),
        Effect::Focused => (
            format!("a focus marker was added to a test: `{}`.", marker.written),
            "a focused case is the only one its file runs, and the rest report as passing.",
        ),
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
            why: why.to_string(),
            next: "take the marker out, or write what stops the test running in a `Weeder-allow: T3` trailer.".to_string(),
        },
        fix: Some(Fix {
            description: "delete the line that carries the marker.".to_string(),
            replacement: None,
        }),
    }
}
