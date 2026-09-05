//! T5 — expected values were regenerated.
//!
//! A snapshot is an expectation nobody typed: the runner wrote it down the
//! first time and compares against it forever after. Accepting a new one is a
//! single keystroke, and it turns a failing test green without anyone deciding
//! that the new output is right.
//!
//! On its own that is housekeeping — a file renamed, a fixture extended — so
//! weed says nothing. It is the pairing that reads badly: the code changed and
//! the expectation moved with it, in the same change, which is the shape of an
//! expectation that was made to agree rather than checked. The finding lands on
//! the expectation and names the production file it moved alongside.

use crate::core::change::Change;
use crate::core::classify::FileKind;
use crate::core::finding::{Finding, Level, Message};

/// The directories a suite keeps its recorded expectations in, whichever
/// runner wrote them.
const EXPECTATION_DIRECTORIES: &[&str] = &[
    "__snapshots__",
    "snapshots",
    "testdata",
    "golden",
    "fixtures",
    "__fixtures__",
];

/// The extensions a recorded expectation carries.
const EXPECTATION_EXTENSIONS: &[&str] = &["snap", "golden", "approved"];

/// The word an approval file carries in front of its extension: `page.approved.txt`.
const APPROVED: &str = "approved";

pub fn evaluate(changes: &[Change]) -> Vec<Finding> {
    let production: Vec<&str> = changes
        .iter()
        .filter(|change| is_production(change))
        .filter_map(|change| change.path())
        .collect();
    let Some(alongside) = production.first() else {
        return Vec::new();
    };

    changes
        .iter()
        .filter_map(|change| {
            let path = change.path()?;
            is_expectation(path).then(|| finding(path, alongside, production.len()))
        })
        .collect()
}

/// Whether the change is to production code. An expectation that happens to be
/// written in a language weed reads is still an expectation, so the paths are
/// asked first.
fn is_production(change: &Change) -> bool {
    change.path().is_some_and(|path| {
        !is_expectation(path)
            && (change.after.is(FileKind::Prod) || change.before.is(FileKind::Prod))
    })
}

/// Whether a path names a recorded expectation: a file in one of the
/// directories a runner writes them to, or one carrying the extension it writes
/// them with.
fn is_expectation(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    let Some((name, directories)) = segments.split_last() else {
        return false;
    };
    if directories
        .iter()
        .any(|directory| EXPECTATION_DIRECTORIES.contains(directory))
    {
        return true;
    }
    let extensions: Vec<&str> = name.split('.').skip(1).collect();
    extensions
        .last()
        .is_some_and(|last| EXPECTATION_EXTENSIONS.contains(last))
        || extensions.contains(&APPROVED)
}

fn finding(path: &str, alongside: &str, production: usize) -> Finding {
    let others = match production {
        1 => String::new(),
        more => format!(" and {} more", more - 1),
    };
    Finding {
        rule: "T5".to_string(),
        level: Level::Warn,
        path: path.to_string(),
        region: None,
        message: Message {
            what: format!(
                "a recorded expectation changed alongside the code it judges: `{alongside}`{others}."
            ),
            why: "an expectation rewritten to match the run it failed on agrees with whatever the code now does.".to_string(),
            next: "read the new expectation as a diff and say in the change why the new output is the right one.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
