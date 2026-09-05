//! T2 — assertions were dropped from a changed test file.
//!
//! The counts the finding has to name are read off the fixture here, by looking
//! for the forms each language's frameworks write an assertion in, so the
//! expectation does not depend on how the detector goes about counting them.
//!
//! The neighbour moves an assertion from one case to another and changes
//! nothing else: the same claims, made in a different place. The allowance is
//! the fire fixture again, this time with a commit message that says why the
//! suite is making fewer claims than it was.

mod common;

use common::{fixture, fixture_file};

/// A language, the test file its assertions live in, and the way that language
/// writes one.
struct Language {
    name: &'static str,
    path: &'static str,
    forms: &'static [&'static str],
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        path: "src/format.test.ts",
        forms: &["expect("],
    },
    Language {
        name: "py",
        path: "tests/test_format.py",
        forms: &["assert "],
    },
    Language {
        name: "rs",
        path: "tests/format.rs",
        forms: &["assert_eq!("],
    },
    Language {
        name: "go",
        path: "format_test.go",
        forms: &["t.Errorf("],
    },
];

/// The reason a change carries when it means to make fewer claims.
const REASON: &str = "the two padding cases became one, and the second claim moved into it";

#[test]
fn t2_fires_at_block_level_when_a_changed_test_file_makes_fewer_claims() {
    for language in LANGUAGES {
        let repo = fixture("T2", language.name, "fire");
        let name = language.name;
        let before = assertions(&language, "fire/before");
        let after = assertions(&language, "fire/after");
        assert!(
            after < before && after > 0,
            "{name}: the fixture has to drop assertions and keep some, not empty the file"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.code, 2,
            "{name}: dropped assertions block\n{}",
            run.stderr
        );

        let findings: Vec<common::Finding> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule == "T2")
            .collect();
        assert_eq!(
            findings.len(),
            1,
            "{name}: one file, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "error", "{name}: T2 blocks");
        assert_eq!(finding.path, language.path, "{name}: the file is named");
        assert!(
            finding.message.contains(&before.to_string())
                && finding.message.contains(&after.to_string()),
            "{name}: the finding names both counts: {}",
            finding.message
        );
    }
}

#[test]
fn t2_stays_silent_when_the_same_claims_are_made_from_another_case() {
    for language in LANGUAGES {
        let repo = fixture("T2", language.name, "silent");
        let name = language.name;

        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path == language.path),
            "{name}: the neighbour must be in the diff, or the silence proves nothing"
        );
        assert_eq!(
            assertions(&language, "silent/before"),
            assertions(&language, "silent/after"),
            "{name}: the neighbour moves an assertion and drops none"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: an assertion that moved is an assertion that is still made"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t2_is_allowed_by_a_trailer_that_carries_a_reason() {
    for language in LANGUAGES {
        let repo = fixture("T2", language.name, "fire");
        let name = language.name;
        repo.pending_message(&format!(
            "Fold the padding cases together\n\nWeed-allow: T2 {REASON}\n"
        ));

        let run = repo.weed(&["check"]);
        let findings = run.findings();
        assert!(
            findings.iter().any(|finding| finding.rule == "T2"),
            "{name}: an allowed finding is still reported"
        );
        assert!(
            findings
                .iter()
                .filter(|finding| finding.rule == "T2")
                .all(|finding| finding.suppressed && finding.level == "note"),
            "{name}: the trailer stands the finding down: {findings:?}"
        );
        assert_eq!(
            run.code, 0,
            "{name}: an allowance a reason travels with stops nobody"
        );
    }
}

#[test]
fn t2_refuses_a_trailer_with_no_reason() {
    let language = &LANGUAGES[0];
    let repo = fixture("T2", language.name, "fire");
    repo.pending_message("Fold the padding cases together\n\nWeed-allow: T2\n");

    let run = repo.weed(&["check"]);
    assert!(
        run.findings()
            .iter()
            .any(|finding| finding.rule == "T2" && !finding.suppressed),
        "a trailer that says nothing allows nothing"
    );
    assert_eq!(run.code, 2, "and the change is still stopped");
}

/// How many assertions a fixture's test file makes, counted from the forms the
/// language writes one in.
fn assertions(language: &Language, case: &str) -> usize {
    let (state, side) = case
        .split_once('/')
        .expect("a fixture case is written as `case/side`");
    let source = fixture_file(
        "T2",
        language.name,
        state,
        &format!("{side}/{}", language.path),
    );
    source
        .lines()
        .map(|line| {
            language
                .forms
                .iter()
                .map(|form| line.matches(form).count())
                .sum::<usize>()
        })
        .sum()
}
