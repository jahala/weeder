//! The cases a change adds, each read for what its assertions are about.
//!
//! T1 and T2 ask what a suite lost. The rules that read this ask the other
//! question, of what a suite gained: whether a case the change wrote can fail
//! at all. A case already committed was judged when it landed, so only a case
//! whose name the file did not carry before is asked, and only one that
//! asserts something: a case with no assertion at all is a different finding.
//!
//! Each assertion is read through [`subject`] for the values it is about, the
//! same reading T2 follows claims with. A line that makes several assertions is
//! read for none of their subjects, because which value each one is about is
//! not a question one line answers, and a rule that cannot read every check of
//! a case says nothing about it.

use std::collections::BTreeSet;

use crate::core::change::{Change, Side};
use crate::core::read::TestUnit;
use crate::core::rules::check::subject::{self, Reading};
use crate::core::rules::check::vocab::Suite;

/// One case the change added, and the assertions it makes.
pub struct Added<'a> {
    /// The file the case was added to.
    pub path: &'a str,
    /// That file as the change left it.
    pub side: &'a Side,
    /// The case itself.
    pub case: &'a TestUnit,
    /// Its assertions, in the order it makes them. Never empty.
    pub checks: Vec<Check>,
}

/// One assertion of an added case.
pub struct Check {
    /// The 1-based line the assertion is made on.
    pub line: u32,
    /// What it is about, where the line can be read for it.
    pub reading: Option<Reading>,
}

/// Every case the change added to a file that holds tests, with what each one
/// asserts, in the order the change and its files are written.
#[must_use]
pub fn cases(changes: &[Change]) -> Vec<Added<'_>> {
    let mut found = Vec::new();
    for change in changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        let side = &change.after;
        let mask = side.mask();
        let suite = Suite::of(side.lang(), mask);
        let cases: Vec<&TestUnit> = side.tests.cases().collect();
        for case in added_cases(change) {
            let checks: Vec<Check> = (case.start_line..=case.end_line)
                .filter(|line| enclosing(&cases, *line) == Some(case))
                .filter_map(|line| {
                    let count = suite.assertions(mask, line);
                    (count > 0).then(|| Check {
                        line,
                        reading: (count == 1)
                            .then(|| subject::read(side.lang(), mask, line, case.start_line))
                            .flatten(),
                    })
                })
                .collect();
            if !checks.is_empty() {
                found.push(Added {
                    path,
                    side,
                    case,
                    checks,
                });
            }
        }
    }
    found
}

/// The cases a changed file holds now under a name it did not hold before. A
/// file the change added holds nothing before, so every case in it is new.
#[must_use]
pub fn added_cases(change: &Change) -> Vec<&TestUnit> {
    if !change.after.holds_tests() {
        return Vec::new();
    }
    let held: BTreeSet<&str> = change
        .before
        .tests
        .cases()
        .map(|case| case.name.as_str())
        .collect();
    change
        .after
        .tests
        .cases()
        .filter(|case| !held.contains(case.name.as_str()))
        .collect()
}

/// The case a line sits in: the narrowest one that spans it, so a case written
/// inside another answers for its own lines.
fn enclosing<'a>(cases: &[&'a TestUnit], line: u32) -> Option<&'a TestUnit> {
    cases
        .iter()
        .filter(|case| case.start_line <= line && line <= case.end_line)
        .min_by_key(|case| case.end_line.saturating_sub(case.start_line))
        .copied()
}
