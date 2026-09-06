//! T5, expected values were regenerated.
//!
//! The fire fixture is the pairing that reads badly: a production file changed
//! and the recorded expectation changed with it, in one commit. Each language
//! keeps its expectations where its own runner writes them, a snapshot
//! directory, a golden file under the data directory, and the fixture puts
//! them there rather than somewhere weed would find convenient.
//!
//! Two neighbours, because there are two innocent halves. One moves the
//! expectation and leaves the code alone: a recording accepted on its own. The
//! other moves the code and leaves the expectation: a change the recording
//! still agrees with.

mod common;

use common::{fixture, Finding, Repo};

/// A language, where its runner keeps the recorded expectation, and the
/// production file that expectation judges.
struct Language {
    name: &'static str,
    expectation: &'static str,
    production: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        expectation: "src/__snapshots__/format.test.ts.snap",
        production: "src/format.ts",
    },
    Language {
        name: "py",
        expectation: "tests/__snapshots__/test_format.ambr",
        production: "src/format.py",
    },
    Language {
        name: "rs",
        expectation: "tests/snapshots/format__pads_to_the_width.snap",
        production: "src/lib.rs",
    },
    Language {
        name: "go",
        expectation: "testdata/format.golden",
        production: "format.go",
    },
];

#[test]
fn t5_warns_on_an_expectation_that_moved_with_the_code_it_judges() {
    for language in LANGUAGES {
        let repo = fixture("T5", language.name, "fire");
        let name = language.name;
        let changed = changed(&repo);
        assert!(
            changed.contains(&language.expectation.to_string())
                && changed.contains(&language.production.to_string()),
            "{name}: the fixture has to move both halves: {changed:?}"
        );

        let run = repo.weed(&["check"]);
        let findings = reported(&run.findings());
        assert_eq!(findings.len(), 1, "{name}: one expectation, one finding");
        let finding = &findings[0];
        assert_eq!(finding.level, "warning", "{name}: T5 warns");
        assert_eq!(
            finding.path, language.expectation,
            "{name}: the finding lands on the expectation"
        );
        assert!(
            finding.message.contains(language.production),
            "{name}: and names the code it moved with: {}",
            finding.message
        );
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn t5_stays_silent_when_only_the_expectation_moved() {
    for language in LANGUAGES {
        let repo = fixture("T5", language.name, "silent");
        let name = language.name;
        assert_eq!(
            changed(&repo),
            vec![language.expectation.to_string()],
            "{name}: the neighbour moves the expectation and nothing else"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a recording accepted on its own is nobody's agreement"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t5_stays_silent_when_only_the_code_moved() {
    for language in LANGUAGES {
        let repo = fixture("T5", language.name, "silent-code");
        let name = language.name;
        let changed = changed(&repo);
        assert!(
            changed.contains(&language.production.to_string())
                && !changed.contains(&language.expectation.to_string()),
            "{name}: the neighbour moves the code and leaves the expectation: {changed:?}"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: code the old expectation still agrees with is code nobody rewrote it for"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// The paths the fixture's change touched, as git sees them.
fn changed(repo: &Repo) -> Vec<String> {
    repo.git(&["diff", "HEAD", "--name-only"])
        .lines()
        .map(str::to_string)
        .collect()
}

fn reported(findings: &[Finding]) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule == "T5")
        .cloned()
        .collect()
}
