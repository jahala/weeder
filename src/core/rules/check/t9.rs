//! T9, a test case was added with no assertion.
//!
//! T2 counts the assertions that leave a changed test file, so a case that
//! arrives with none passes it by construction: nothing left. A suite can grow
//! by a hundred green cases that call the unit and check nothing, and every
//! count weeder prints stays honest while it happens.
//!
//! weeder reports a case the change added where nothing in it checks a
//! result. A case asserts when one of its own lines makes an assertion in the
//! vocabulary T2 counts with, the helpers `[tests] assertions` names among
//! them; when it is declared to expect a failure; when it is declared to
//! return a failure and hands one back; or when it calls a function the same
//! file defines whose own lines assert. That last reading goes one level and
//! no deeper: the outline names every function of the file and its lines, so
//! one level costs nothing, and a chain of helpers is the repository's to name
//! under `[tests] assertions`.
//!
//! The rule warns. A case that only proves a unit runs without panicking is an
//! honest test in Rust and Go, and only the person at the pull request can
//! tell it from one that forgot to check.

use crate::core::change::Side;
use crate::core::finding::{Detection, Message, Region, Stamp};
use crate::core::read::{Definition, DefinitionKind, TestUnit};
use crate::core::rules::check::added::added_cases;
use crate::core::rules::check::vocab::{names, Suite};
use crate::core::rules::check::Judgement;

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let helpers = &judged.config.test_assertions;
    let mut found = Vec::new();
    for change in judged.changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        let added = added_cases(change);
        if added.is_empty() {
            continue;
        }
        let side = &change.after;
        let file = File {
            side,
            suite: Suite::of(side.lang(), side.mask()).counting(helpers),
            functions: side
                .outline
                .flatten()
                .into_iter()
                .filter(|definition| definition.kind == DefinitionKind::Function)
                .collect(),
        };
        found.extend(
            added
                .into_iter()
                .filter(|case| !file.asserts(case))
                .map(|case| finding(path, case)),
        );
    }
    found
}

/// A file the change added cases to, read once for all of them: its
/// vocabulary, and the functions its outline names.
struct File<'a> {
    side: &'a Side,
    suite: Suite,
    functions: Vec<&'a Definition>,
}

impl File<'_> {
    /// Whether anything in the case checks a result.
    fn asserts(&self, case: &TestUnit) -> bool {
        let mask = self.side.mask();
        let lines = self.own_lines(case);
        if lines
            .iter()
            .any(|line| self.suite.assertions(mask, *line) > 0)
            || self.suite.expects_failure(case)
        {
            return true;
        }
        let returns_failure = self
            .functions
            .iter()
            .find(|function| function.start_line == case.start_line && function.name == case.name)
            .and_then(|function| function.signature.as_deref())
            .is_some_and(|signature| self.suite.returns_failure(signature));
        if returns_failure
            && lines
                .iter()
                .any(|line| self.suite.hands_back(mask, *line) > 0)
        {
            return true;
        }
        self.calls_an_asserting_helper(case, &lines)
    }

    /// The case's own lines: inside it, and in no narrower case written
    /// inside it.
    fn own_lines(&self, case: &TestUnit) -> Vec<u32> {
        (case.start_line..=case.end_line)
            .filter(|line| self.side.tests.case_at(*line) == Some(case))
            .collect()
    }

    /// Whether the case calls a function its file defines outside the case,
    /// and that function's own lines assert. The helper is read for the
    /// vocabulary alone, one level deep.
    fn calls_an_asserting_helper(&self, case: &TestUnit, lines: &[u32]) -> bool {
        let mask = self.side.mask();
        lines.iter().any(|line| {
            names(&mask.code(*line))
                .iter()
                .filter(|name| name.is_called())
                .any(|called| {
                    self.functions.iter().any(|function| {
                        function.name == called.last()
                            && !function.spans(case.start_line)
                            && (function.start_line..=function.end_line)
                                .any(|inside| self.suite.assertions(mask, inside) > 0)
                    })
                })
        })
    }
}

fn finding(path: &str, case: &TestUnit) -> Detection {
    Detection {
        stamp: Stamp::Rule,
        path: path.to_string(),
        region: Some(Region {
            start_line: case.start_line,
            end_line: case.start_line,
        }),
        message: Message {
            what: format!(
                "the added case `{}` asserts nothing: no line of it checks a result, and it calls no helper of this file that does.",
                case.name
            ),
            why: "a case that checks nothing reports green over behaviour nobody holds, so the suite grows and proves nothing new.".to_string(),
            next: "assert what the case is for, or write why a run that checks nothing is the point in a `Weeder-allow: T9` trailer.".to_string(),
        },
        fix: None,
    }
}
