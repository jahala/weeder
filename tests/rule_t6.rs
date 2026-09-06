//! T6, an error assertion was weakened.
//!
//! Each language gets two shapes: an assertion that named the message the code
//! raises, and one that named the kind. Both come back from the change saying
//! only that something failed. The lines weeder must report are read off the
//! fixture by comparing its two sides, so the expectation follows the change
//! rather than the detector.
//!
//! The neighbour is the same fixture the other way round, an assertion that
//! arrives naming what it used to accept blindly, which is the edit this rule
//! must never punish.

mod common;

use common::{fixture, fixture_file, Finding};

/// A language and the test file its error assertions live in.
struct Language {
    name: &'static str,
    path: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        path: "src/format.test.ts",
    },
    Language {
        name: "py",
        path: "tests/test_format.py",
    },
    Language {
        name: "rs",
        path: "tests/format.rs",
    },
    Language {
        name: "go",
        path: "format_test.go",
    },
];

#[test]
fn t6_warns_on_every_error_assertion_a_change_loosened() {
    for language in LANGUAGES {
        let repo = fixture("T6", language.name, "fire");
        let name = language.name;
        let rewritten = rewritten(&language, "fire");
        assert_eq!(
            rewritten.len(),
            2,
            "{name}: the fixture loosens the message and the kind, and touches nothing else"
        );

        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings());
        assert_eq!(
            findings
                .iter()
                .map(|finding| finding.line.expect("a T6 finding names its line"))
                .collect::<Vec<u64>>(),
            rewritten,
            "{name}: every loosened assertion is reported, and only those"
        );
        for finding in &findings {
            assert_eq!(finding.level, "warning", "{name}: T6 warns");
            assert_eq!(finding.path, language.path, "{name}: the file is named");
        }
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn t6_stays_silent_when_an_assertion_arrives_naming_what_it_accepts() {
    for language in LANGUAGES {
        let repo = fixture("T6", language.name, "silent");
        let name = language.name;
        assert_eq!(
            rewritten(&language, "silent").len(),
            2,
            "{name}: the neighbour rewrites the same two assertions the other way round"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: an assertion that got narrower is the change this rule asks for"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// The lines a fixture's change rewrote, counted from one.
fn rewritten(language: &Language, case: &str) -> Vec<u64> {
    let before = fixture_file(
        "T6",
        language.name,
        case,
        &format!("before/{}", language.path),
    );
    let after = fixture_file(
        "T6",
        language.name,
        case,
        &format!("after/{}", language.path),
    );
    let before: Vec<&str> = before.lines().collect();
    let after: Vec<&str> = after.lines().collect();
    assert_eq!(
        before.len(),
        after.len(),
        "{}/{case}: the two sides have to line up, or the lines mean nothing",
        language.name
    );
    before
        .iter()
        .zip(&after)
        .enumerate()
        .filter(|(_, (was, now))| was != now)
        .map(|(index, _)| index as u64 + 1)
        .collect()
}

fn reported(findings: &[Finding]) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule == "T6")
        .cloned()
        .collect()
}
