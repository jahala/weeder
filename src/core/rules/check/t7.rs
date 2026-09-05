//! T7, a rename took a test out of the runner.
//!
//! Every runner decides what to run by name. A file the naming convention no
//! longer matches is never collected, and a case whose name stopped fitting is
//! never called: the code is still in the tree, the diff shows a rename and
//! nothing else, and the suite quietly got smaller. That is a deleted test
//! wearing the file it used to live in, which is why this blocks.
//!
//! Two shapes, because runners collect in two ways. A file is collected by its
//! path, the suffix its runner globs for, the directory it has to sit in, so
//! a rename that leaves the convention is the finding. A case is collected by
//! its name where the language has no other way to mark one, and a rename that
//! leaves that convention is the same finding one level down. Where a case is
//! marked rather than named, an attribute above it, a title handed to a call ,
//! renaming it collects the test as before, and weed says nothing.

use crate::core::change::{Change, Side};
use crate::core::classify::Lang;
use crate::core::diff::ChangeKind;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::read::{Definition, DefinitionKind};
use crate::core::rules::check::Judgement;
use crate::core::syntax::Mask;

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let changes = judged.changes;
    let mut findings = Vec::new();
    for change in changes {
        findings.extend(uncollected_file(change));
        findings.extend(uncollected_cases(change));
    }
    findings
}

/// A file the change renamed out of what its runner collects.
fn uncollected_file(change: &Change) -> Option<Finding> {
    if change.diff.change != ChangeKind::Renamed {
        return None;
    }
    let (from, to) = (
        change.diff.old_path.as_deref()?,
        change.diff.new_path.as_deref()?,
    );
    let lang = language(&change.before).or_else(|| language(&change.after))?;
    if !collects_file(lang, from) || collects_file(lang, to) {
        return None;
    }
    if !change.before.holds_tests() {
        return None;
    }
    Some(file_finding(change, from, to))
}

/// Every case the change renamed out of what its runner collects.
fn uncollected_cases(change: &Change) -> Vec<Finding> {
    let Some(path) = change.diff.new_path.as_deref() else {
        return Vec::new();
    };
    let Some(lang) = language(&change.after) else {
        return Vec::new();
    };
    if !collects_case_by_name(lang) || !change.before.holds_tests() {
        return Vec::new();
    }

    let before = change.before.mask();
    let after = change.after.mask();
    let was: Vec<&Definition> = functions(&change.before);
    let now: Vec<&Definition> = functions(&change.after);

    let mut findings = Vec::new();
    for gone in was
        .iter()
        .filter(|definition| collects_case(lang, &definition.name))
        .filter(|definition| !now.iter().any(|kept| kept.name == definition.name))
    {
        let held = body(&before, gone);
        let Some(renamed) = now
            .iter()
            .filter(|definition| !collects_case(lang, &definition.name))
            .filter(|definition| !was.iter().any(|kept| kept.name == definition.name))
            // The same body under another name is the same test: a case that
            // was rewritten as well as renamed is a case weed will not claim to
            // have followed.
            .find(|definition| body(&after, definition) == held)
        else {
            continue;
        };
        findings.push(case_finding(path, &gone.name, renamed));
    }
    findings
}

/// Whether the runner collects a file at this path.
fn collects_file(lang: Lang, path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    let Some((name, directories)) = segments.split_last() else {
        return false;
    };
    match lang {
        // The runners glob for a file whose name carries the word in front of
        // its extension, or for anything inside the directory kept for tests.
        Lang::TypeScript | Lang::JavaScript => {
            name.contains(".test.") || name.contains(".spec.") || directories.contains(&"__tests__")
        }
        Lang::Python => {
            name.starts_with("test_")
                || name
                    .strip_suffix(".py")
                    .is_some_and(|stem| stem.ends_with("_test"))
        }
        Lang::Go => name.ends_with("_test.go"),
        // An integration target is a file at the top of the tests directory,
        // and nowhere below it: a file one directory down is a module nobody
        // compiles until something declares it.
        Lang::Rust => {
            directories == ["tests"]
                || directories == ["benches"]
                || directories.first() == Some(&"src")
        }
        Lang::Other => false,
    }
}

/// Whether the language's runner decides what a case is from its name alone.
fn collects_case_by_name(lang: Lang) -> bool {
    matches!(lang, Lang::Python | Lang::Go)
}

/// Whether the runner collects a case of this name.
fn collects_case(lang: Lang, name: &str) -> bool {
    match lang {
        Lang::Python => name == "test" || name.starts_with("test_"),
        // The prefix says which kind of run it is, and something has to follow
        // it: `Test` on its own is a helper.
        Lang::Go => ["Test", "Benchmark", "Fuzz", "Example"]
            .iter()
            .any(|prefix| {
                name.strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with(char::is_uppercase))
            }),
        _ => false,
    }
}

/// The functions a side declares, at every depth.
fn functions(side: &Side) -> Vec<&Definition> {
    side.outline
        .flatten()
        .into_iter()
        .filter(|definition| definition.kind == DefinitionKind::Function)
        .collect()
}

/// What a declaration holds, without the line that names it and without the
/// spacing, so the same test under another name compares equal.
fn body(mask: &Mask, definition: &Definition) -> String {
    (definition.start_line.saturating_add(1)..=definition.end_line)
        .map(|line| mask.outside_comments(line))
        .map(|line| line.split_whitespace().collect::<Vec<&str>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<String>>()
        .join("\n")
}

fn language(side: &Side) -> Option<Lang> {
    side.classification
        .as_ref()
        .map(|classification| classification.lang)
        .filter(|lang| *lang != Lang::Other)
}

fn file_finding(change: &Change, from: &str, to: &str) -> Finding {
    let cases = change.before.case_count();
    Finding {
        rule: "T7".to_string(),
        level: Level::Block,
        path: to.to_string(),
        region: change.first_edit().map(|line| Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!(
                "a test file was renamed out of what the runner collects: `{from}` became `{to}`, and {} went with it.",
                counted(cases)
            ),
            why: "a file the runner never opens reports nothing, and the tests inside it still read as present.".to_string(),
            next: "rename it back inside the convention, or move its cases into a file the runner collects.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

fn case_finding(path: &str, from: &str, renamed: &Definition) -> Finding {
    Finding {
        rule: "T7".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: Some(Region {
            start_line: renamed.start_line,
            end_line: renamed.start_line,
        }),
        message: Message {
            what: format!(
                "a test case was renamed out of what the runner collects: `{from}` became `{}`.",
                renamed.name
            ),
            why: "the body is still here and nothing calls it, so the case reads as present and never runs.".to_string(),
            next: "give it back a name the runner collects, or call it from a case that has one.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

fn counted(cases: usize) -> String {
    if cases == 1 {
        "1 test case".to_string()
    } else {
        format!("{cases} test cases")
    }
}
