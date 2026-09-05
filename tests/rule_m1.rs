//! M1, a test mocks the unit under change.
//!
//! Each fire fixture changes a production file and, in the same commit, a test
//! that stands a double in front of it. The languages name their doubles two
//! ways: the two that resolve a module hand the framework a specifier, a
//! relative path, a dotted module, and the two that resolve a type give the
//! double the name of what it replaces.
//!
//! The neighbour is the same change with the double pointed at a unit the
//! commit does not touch. The doubled file is in the repository both times, so
//! the difference between the two fixtures is what the change reaches, and
//! nothing else.

mod common;

use common::{fixture, fixture_file, Finding, Repo};

/// A language, the test file its double is set up in, the production file the
/// change edits, what a finding has to call the double, and the line each
/// fixture sets one up with.
struct Language {
    name: &'static str,
    test: &'static str,
    production: &'static str,
    named: &'static str,
    inside: &'static str,
    outside: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        test: "src/format.test.ts",
        production: "src/format.ts",
        named: "./format",
        inside: "vi.mock(\"./format\")",
        outside: "vi.mock(\"./clock\")",
    },
    Language {
        name: "py",
        test: "tests/test_format.py",
        production: "src/format.py",
        named: "src.format.format_value",
        inside: "@patch(\"src.format.format_value\")",
        outside: "@patch(\"src.clock.now\")",
    },
    Language {
        name: "rs",
        test: "tests/format.rs",
        production: "src/width.rs",
        named: "Width",
        inside: "MockWidth::new()",
        outside: "MockClock::new()",
    },
    Language {
        name: "go",
        test: "format_test.go",
        production: "format.go",
        named: "Formatter",
        inside: "NewMockFormatter(t)",
        outside: "NewMockClock(t)",
    },
];

#[test]
fn m1_warns_when_the_double_stands_in_for_a_file_the_change_edits() {
    for language in LANGUAGES {
        let repo = fixture("M1", language.name, "fire");
        let name = language.name;
        let changed = changed(&repo);
        assert!(
            changed.contains(&language.test.to_string())
                && changed.contains(&language.production.to_string()),
            "{name}: the change has to hold both the test and the unit: {changed:?}"
        );
        assert!(
            names_the_double(&language, "fire", language.inside),
            "{name}: the fire fixture has to double the unit the change edits"
        );

        let run = repo.weed(&["check"]);
        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one double, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "warning", "{name}: M1 warns");
        assert_eq!(
            finding.path, language.test,
            "{name}: the finding lands on the test"
        );
        assert!(
            finding.message.contains(language.named)
                && finding.message.contains(language.production),
            "{name}: the finding names the double and the file it stands in for: {}",
            finding.message
        );
        assert!(
            finding.line.is_some_and(|line| line > 0),
            "{name}: and the line the double was set up on"
        );
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn m1_stays_silent_when_the_double_stands_in_for_something_outside_the_change() {
    for language in LANGUAGES {
        let repo = fixture("M1", language.name, "silent");
        let name = language.name;
        let changed = changed(&repo);
        assert!(
            changed.contains(&language.test.to_string())
                && changed.contains(&language.production.to_string()),
            "{name}: the neighbour changes the same two files: {changed:?}"
        );
        assert!(
            names_the_double(&language, "silent", language.outside)
                && !names_the_double(&language, "silent", language.inside),
            "{name}: and the double it sets up stands in for something else entirely"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a double of something the change never reaches is somebody's decision, not a finding"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// Whether a fixture's test file names that double after the change.
fn names_the_double(language: &Language, case: &str, double: &str) -> bool {
    fixture_file(
        "M1",
        language.name,
        case,
        &format!("after/{}", language.test),
    )
    .contains(double)
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
        .filter(|finding| finding.rule == "M1")
        .cloned()
        .collect()
}
