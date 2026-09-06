//! B1, a test passed without the change it covers.
//!
//! The command the caller named passed with the test commit alone applied to
//! the base. Whatever that commit added was therefore green before the
//! implementation existed, and a green test proves nothing about code that was
//! not there when it ran.
//!
//! Which tests those are is read from the test shape of each side of the change,
//! never from the runner's output: a runner prints what it likes, and the cases
//! a commit added are a property of the files it changed. A file whose language
//! weeder reads no tests in is still reported, by the file rather than by the
//! case, because the command passed over it all the same.

use std::collections::BTreeSet;

use crate::core::bite::{Trial, Verdict};
use crate::core::change::Change;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::read::TestUnit;

pub fn evaluate(trial: &Trial) -> Vec<Finding> {
    if trial.alone == Verdict::Failed {
        return Vec::new();
    }
    subject(trial.tested)
        .into_iter()
        .flat_map(reported)
        .collect()
}

/// The files a finding is written against. A test commit carries tests, so
/// those are what is reported; where it carries none weeder can read, every file
/// it changed is named instead, because the command passed over all of them and
/// none of it was the implementation.
fn subject(tested: &[Change]) -> Vec<&Change> {
    let holding: Vec<&Change> = tested
        .iter()
        .filter(|change| change.after.holds_tests() || !added_cases(change).is_empty())
        .collect();
    if holding.is_empty() {
        tested.iter().collect()
    } else {
        holding
    }
}

/// What one file has to answer for: a finding per case the commit added, or one
/// for the file where weeder can name no case in it.
fn reported(change: &Change) -> Vec<Finding> {
    let Some(path) = change.path() else {
        return Vec::new();
    };
    let added = added_cases(change);
    if added.is_empty() {
        return vec![whole_file(path)];
    }
    added.into_iter().map(|case| one_case(path, case)).collect()
}

/// The cases this change added: a case on the after side the before side did
/// not declare. A case that was already there ran before the test commit and
/// says nothing about it.
fn added_cases(change: &Change) -> Vec<&TestUnit> {
    let before: BTreeSet<&str> = change
        .before
        .tests
        .cases()
        .map(|case| case.name.as_str())
        .collect();
    change
        .after
        .tests
        .cases()
        .filter(|case| !before.contains(case.name.as_str()))
        .collect()
}

fn one_case(path: &str, case: &TestUnit) -> Finding {
    Finding {
        rule: "B1".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: Some(Region {
            start_line: case.start_line,
            end_line: case.end_line,
        }),
        message: Message {
            what: format!(
                "the case `{}` passed with the test commit alone applied to the base.",
                case.name
            ),
            why: "a test that is green before its change exists holds the change to nothing, so it can neither fail nor be trusted."
                .to_string(),
            next: "make the case fail on the base, by asserting the behaviour the change adds, and run bite again.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

fn whole_file(path: &str) -> Finding {
    Finding {
        rule: "B1".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: None,
        message: Message {
            what: "the test command passed with the test commit alone applied to the base, and this is a file that commit changed."
                .to_string(),
            why: "a suite that is green before the change exists holds it to nothing, so it can neither fail nor be trusted."
                .to_string(),
            next: "make the tests this commit carries fail on the base, then run bite again.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
