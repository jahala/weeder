//! T5, expected values were regenerated.
//!
//! A snapshot is an expectation nobody typed: the runner wrote it down the
//! first time and compares against it forever after. Accepting a new one is a
//! single keystroke, and it turns a failing test green without anyone deciding
//! that the new output is right.
//!
//! On its own that is housekeeping, a file renamed, a fixture extended, so
//! weeder says nothing. It is the pairing that reads badly: the code changed and
//! the expectation moved with it, in the same change, which is the shape of an
//! expectation that was made to agree rather than checked. The finding lands on
//! the expectation and names the production file it moved alongside.
//!
//! Two kinds of file live where expectations live and are none. A file the
//! change adds was never agreed with anything: it was born with the test that
//! reads it, which is the order the law asks for. And a test double, a program
//! the tests run in place of a real one, changes with the code by design and
//! holds no recorded output to drift. weeder knows a program by what makes one:
//! the execute bit git records, or a shebang on the first line.

use crate::core::change::Change;
use crate::core::classify::FileKind;
use crate::core::finding::{Detection, Message, Stamp};
use crate::core::rules::check::Judgement;

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

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let changes = judged.changes;
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
        .filter(|change| !change.is_addition() && !is_program(change))
        .filter_map(|change| {
            let path = change.path()?;
            is_expectation(path).then(|| finding(path, alongside, production.len()))
        })
        .collect()
}

/// The mode git records for a file it will run.
const EXECUTABLE: &str = "100755";

/// What a script opens with to name the program that runs it.
const SHEBANG: &str = "#!";

/// Whether the file is a program rather than a record: git keeps it executable,
/// or its first line names an interpreter. The side that still has the file is
/// the one asked, so a double the change deletes is still a double.
fn is_program(change: &Change) -> bool {
    let mode = change
        .diff
        .new_mode
        .as_deref()
        .or(change.diff.old_mode.as_deref());
    if mode == Some(EXECUTABLE) {
        return true;
    }
    let side = if change.after.content.is_some() {
        &change.after
    } else {
        &change.before
    };
    side.text().starts_with(SHEBANG)
}

/// Whether the change is to production code. An expectation that happens to be
/// written in a language weeder reads is still an expectation, so the paths are
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

fn finding(path: &str, alongside: &str, production: usize) -> Detection {
    let others = match production {
        1 => String::new(),
        more => format!(" and {} more", more - 1),
    };
    Detection {
        stamp: Stamp::Rule,
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
    }
}
