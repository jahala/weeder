//! T10, an added case holds a value the code defines to a literal.
//!
//! `expect(SERVICE).toBe("build.example.app")` is the code written out a
//! second time. The constant and the literal say the same thing, so the case
//! changes whenever the code does and catches nothing the code does with the
//! value: a command nobody registers, a host nobody connects to, stays green.
//!
//! weeder reports a case the change added whose every check compares one bare
//! name, read and never called, with a literal, where the name is the code's
//! and not the case's own: the file imports it, the file defines it outside
//! its tests, or the language hands it to the test without an import at all,
//! Go's package scope and a glob import. A name the case binds, takes as a
//! parameter or the test file defines is the test's own expectation, and a
//! name handed to the code with the answer checked against a literal is the
//! honest neighbour: something the code computes stands between them.
//!
//! An expected value imported from the code and compared with what the code
//! returns, `expect(status()).toBe(Status.Ready)`, is left alone. It has an
//! honest reading too common to warn on: the case checks which value comes
//! back, and the import only names it.

use crate::core::change::Side;
use crate::core::classify::{FileKind, Lang};
use crate::core::finding::{Detection, Message, Region, Stamp};
use crate::core::read::DefinitionKind;
use crate::core::rules::check::added::{self, Added};
use crate::core::rules::check::subject::Reading;
use crate::core::rules::check::vocab::names;
use crate::core::rules::check::Judgement;
use crate::core::syntax::is_identifier;

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    added::cases(judged.changes)
        .iter()
        .filter_map(|added| {
            let echoed: Vec<String> = added
                .checks
                .iter()
                .map(|check| echo(added, check.reading.as_ref()?))
                .collect::<Option<Vec<String>>>()?;
            let line = added.checks.first()?.reading.as_ref()?.line;
            Some(finding(added, &echoed, line))
        })
        .collect()
}

/// The name a check restates, where all it does is hold a value the code
/// defines to a literal.
fn echo(added: &Added, reading: &Reading) -> Option<String> {
    if !reading.exact || !reading.against_literal {
        return None;
    }
    let [subject] = reading.subjects.as_slice() else {
        return None;
    };
    if subject.bound.is_some() {
        return None;
    }
    let written = subject.written.as_str();
    if !written
        .chars()
        .all(|character| is_identifier(character) || character == '.' || character == ':')
    {
        return None;
    }
    let root: String = written
        .chars()
        .take_while(|character| is_identifier(*character))
        .collect();
    let last = written.rsplit(['.', ':']).next().unwrap_or(written);
    let owned = |name: &str| !name.is_empty() && from_the_code(added, name);
    (owned(&root) || owned(last)).then(|| written.to_string())
}

/// Whether a name the case reads comes from the code rather than from the case
/// or its file.
fn from_the_code(added: &Added, name: &str) -> bool {
    let side = added.side;
    // A name on the line the case opens with is a parameter of the case: a
    // fixture the runner hands it, the receiver of a method, the handle.
    let opening = side.mask().code(added.case.start_line);
    if names(&opening)
        .iter()
        .any(|written| written.segments.contains(&name))
    {
        return false;
    }
    imported(side, name) || defined_beside_its_tests(side, name) || in_open_scope(side, name)
}

/// Whether an import statement of the file names it.
fn imported(side: &Side, name: &str) -> bool {
    let mask = side.mask();
    let statements = side
        .imports
        .iter()
        .map(|import| (import.start_line, import.end_line))
        .chain(
            side.outline
                .flatten()
                .into_iter()
                .filter(|definition| definition.kind == DefinitionKind::Import)
                .map(|definition| (definition.start_line, definition.end_line)),
        );
    statements
        .flat_map(|(first, last)| first..=last)
        .any(|line| {
            names(&mask.code(line))
                .iter()
                .any(|written| written.segments.contains(&name))
        })
}

/// Whether a production file that carries its own tests defines the name
/// outside them.
fn defined_beside_its_tests(side: &Side, name: &str) -> bool {
    side.is(FileKind::Prod)
        && side.outline.flatten().iter().any(|definition| {
            definition.kind != DefinitionKind::Import
                && definition.name == name
                && !side.tests.units.iter().any(|unit| {
                    unit.start_line <= definition.start_line
                        && definition.start_line <= unit.end_line
                })
        })
}

/// Whether the language hands the test names nobody imports by name, and this
/// is one of them: a Go test sees its whole package, and a glob import brings
/// in whatever the module holds. The file itself must not define the name,
/// and in Go it must not be a package the file imports either.
fn in_open_scope(side: &Side, name: &str) -> bool {
    let open = match side.lang() {
        Lang::Go => true,
        Lang::Python => side
            .imports
            .iter()
            .any(|import| import.text.trim_end().ends_with("import *")),
        Lang::Rust => side
            .imports
            .iter()
            .any(|import| import.source.ends_with("::*")),
        Lang::TypeScript | Lang::JavaScript | Lang::Other => false,
    };
    open && !side
        .outline
        .flatten()
        .iter()
        .any(|definition| definition.kind != DefinitionKind::Import && definition.name == name)
        && !side
            .imports
            .iter()
            .any(|import| side.lang() == Lang::Go && import.source.rsplit('/').next() == Some(name))
}

fn finding(added: &Added, echoed: &[String], line: u32) -> Detection {
    let mut named: Vec<String> = echoed.iter().map(|name| format!("`{name}`")).collect();
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
                "the added case `{}` holds {}, which the code under test defines, to a literal, and checks nothing else.",
                added.case.name,
                named.join(" and ")
            ),
            why: "the case restates the code, so it changes whenever the code does and stays green whatever the code does with the value.".to_string(),
            next: "check something the code computes from the value, or say at the pull request why the value itself is the contract.".to_string(),
        },
        fix: None,
    }
}
