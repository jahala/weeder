//! T11, an added case asserts on nothing but a double it built itself.
//!
//! A double is there to stand in for a collaborator while the unit runs. A
//! case that builds one, tells it what to answer, and then checks that it
//! answered has run nothing but the double: `vi.fn().mockReturnValue(true)`
//! followed by `expect(save("draft")).toBe(true)` passes whatever the code
//! under test does, because the code under test is never called.
//!
//! weeder reports a case the change added where every assertion reads a
//! double the same case bound, or a value the case read off one, and the case
//! hands that double to nothing. Configuring it, `save.mockReturnValue(…)` or
//! `clock.set(5)`, and calling it are not handing it on; passing it as an
//! argument or a value is, and from then on an assertion on the double reads
//! what the unit did to it, which is the effect a double is for.
//!
//! Which names make a double is the vocabulary M1 reads doubles by, and the
//! script frameworks' mock function besides; see [`makes_a_double`].

use std::collections::BTreeMap;

use crate::core::classify::Lang;
use crate::core::finding::{Detection, Message, Region, Stamp};
use crate::core::rules::check::added::{self, Added, Check};
use crate::core::rules::check::generated::assigned_at;
use crate::core::rules::check::vocab::{makes_a_double, names, Suite};
use crate::core::rules::check::Judgement;
use crate::core::syntax::{is_identifier, words};

/// The words that may stand in front of what a value is made by, and say
/// nothing about it: `new Editor(…)`, `&mut FakeClock::new()`, `await make()`.
const LEADING_WORDS: &[&str] = &["new", "await", "mut", "ref", "move"];

/// The words written before a name a line binds: the ones that declare it,
/// and Go's statements that bind one in their header, `if got := …; …`.
const DECLARING_WORDS: &[&str] = &["const", "let", "var", "mut", "if", "for"];

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    added::cases(judged.changes)
        .iter()
        .filter_map(|added| {
            let doubles = doubles(added);
            if doubles.is_empty() {
                return None;
            }
            let suite = Suite::of(added.side.lang(), added.side.mask());
            let read: Vec<String> = added
                .checks
                .iter()
                .map(|check| reads(added, &suite, check, &doubles))
                .collect::<Option<Vec<String>>>()?;
            if read.iter().any(|double| handed(added, double, &doubles)) {
                return None;
            }
            let first = added.checks.first()?;
            let line = first
                .reading
                .as_ref()
                .map_or(first.line, |reading| reading.line);
            Some(finding(added, &read, line))
        })
        .collect()
}

/// The names the case binds to a double, each with the line it binds it on.
fn doubles(added: &Added) -> BTreeMap<String, u32> {
    let mask = added.side.mask();
    let lang = added.side.lang();
    let mut found = BTreeMap::new();
    for line in added.lines() {
        let code = mask.code(line);
        let Some((name, value)) = binding(&code) else {
            continue;
        };
        if made_by_a_double(lang, value) {
            found.entry(name.to_string()).or_insert(line);
        }
    }
    found
}

/// The name a line binds and the code of what it binds it to, where the line
/// binds a plain name at all. A member written to, `self.fake = …`, binds no
/// name the case can read on its own, and a type written after the name,
/// `const save: Mock = …`, is not what is bound.
fn binding(code: &str) -> Option<(&str, &str)> {
    let equals = code.find('=')?;
    let bound = words(&code[..equals])
        .into_iter()
        .find(|word| !DECLARING_WORDS.contains(&word.text))
        .filter(|word| !word.is_member() && word.after != Some('.'))?;
    let column = assigned_at(code, bound.text)?;
    let start = code
        .char_indices()
        .nth(column)
        .map_or(code.len(), |(at, _)| at);
    Some((bound.text, &code[start..]))
}

/// Whether a value is made by something that makes a double: the first name
/// it is built from, past the words that only say how.
fn made_by_a_double(lang: Lang, value: &str) -> bool {
    names(value)
        .iter()
        .find(|name| !(name.segments.len() == 1 && LEADING_WORDS.contains(&name.text)))
        .is_some_and(|name| makes_a_double(lang, name))
}

/// The double a check reads, where every value it reads is one the case bound
/// to a double or read off one.
fn reads(
    added: &Added,
    suite: &Suite,
    check: &Check,
    doubles: &BTreeMap<String, u32>,
) -> Option<String> {
    let bound_before = |name: &str| {
        doubles
            .get(name)
            .is_some_and(|line| *line < check.line)
            .then(|| name.to_string())
    };
    match &check.reading {
        Some(reading) => {
            let found: Vec<String> = reading
                .subjects
                .iter()
                .map(|subject| {
                    bound_before(&root(&subject.written)).or_else(|| {
                        subject.bound.as_deref().and_then(|value| {
                            bound_before(&root(value.trim_start_matches(['&', '*', ' '])))
                        })
                    })
                })
                .collect::<Option<Vec<String>>>()?;
            found.into_iter().next()
        }
        // An assertion a reading cannot take apart, `notify.assert_called_once()`,
        // is about the value it is called on.
        None => {
            let code = added.side.mask().code(check.line);
            let asserting = suite.asserting(&code);
            let [name] = asserting.as_slice() else {
                return None;
            };
            (name.segments.len() > 1)
                .then(|| name.segments.first().copied())
                .flatten()
                .and_then(bound_before)
        }
    }
}

/// The name an expression begins with.
fn root(expression: &str) -> String {
    expression
        .trim_start()
        .chars()
        .take_while(|character| is_identifier(*character))
        .collect()
}

/// Whether the case hands the double to anything: writes it anywhere but
/// where it binds it, configures it, calls it, or checks it.
fn handed(added: &Added, double: &str, doubles: &BTreeMap<String, u32>) -> bool {
    let mask = added.side.mask();
    let bound_on = doubles.get(double).copied();
    added
        .lines()
        .into_iter()
        .filter(|line| Some(*line) != bound_on && !added.checks_on(*line))
        .any(|line| {
            let code = mask.code(line);
            words(&code).iter().any(|word| {
                word.text == double
                    && !word.is_member()
                    && !matches!(word.after, Some('.' | '(' | '[' | ':' | '='))
            })
        })
}

fn finding(added: &Added, read: &[String], line: u32) -> Detection {
    let mut named: Vec<String> = read.iter().map(|name| format!("`{name}`")).collect();
    named.sort();
    named.dedup();
    Detection {
        stamp: Stamp::Rule,
        path: added.path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!(
                "the added case `{}` asserts only on {}, a double it built itself and hands to nothing.",
                added.case.name,
                named.join(" and ")
            ),
            why: "every check reads back what the case told the double to do, so the case passes whatever the code under test does.".to_string(),
            next: "hand the double to the unit and check what the unit does, or check the unit's own output.".to_string(),
        },
        fix: None,
    }
}
