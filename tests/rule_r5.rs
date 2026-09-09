//! R5, a test file the configuration never collects.
//!
//! T8 judges the line that hides a suite, which only helps where somebody is
//! there to see the change. A repository that arrived with the exclusion
//! already in it has nobody to stop, so `scan` reads the same collector against
//! the tree as it sits and names every suite the runner will not open.
//!
//! Each fire fixture holds two suites, one the configuration reaches and one it
//! does not, so a rule that reported the whole directory, or nothing, would
//! fail here. Each silent fixture holds a configuration that reaches both.

mod common;

use common::{fixture, Finding, Repo};

/// A language, the suite its configuration never collects, the suite it does,
/// and the file that decides.
struct Language {
    name: &'static str,
    hidden: &'static str,
    collected: &'static str,
    by: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        hidden: "src/format.test.ts",
        collected: "src/parse.test.ts",
        by: "jest.config.js",
    },
    Language {
        name: "py",
        hidden: "tests/test_format.py",
        collected: "tests/test_parse.py",
        by: "tests/conftest.py",
    },
    Language {
        name: "rs",
        hidden: "tests/format.rs",
        collected: "tests/parse.rs",
        by: "tests/format.rs",
    },
    Language {
        name: "go",
        hidden: "format_test.go",
        collected: "parse_test.go",
        by: "format_test.go",
    },
];

#[test]
fn r5_names_every_test_file_the_configuration_never_collects() {
    for language in LANGUAGES {
        let name = language.name;
        let repo = fixture("R5", name, "fire");
        for suite in [language.hidden, language.collected] {
            assert!(
                holds(&repo, suite),
                "{name}: the tree has to hold {suite} for the reading to mean anything"
            );
        }

        let run = repo.weeder(&["scan"]);
        assert_eq!(run.code, 0, "{name}: a scan never blocks\n{}", run.stderr);

        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one suite is out of the run, and one finding says so: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "warning", "{name}: R5 reports, never blocks");
        assert_eq!(
            finding.path, language.hidden,
            "{name}: one finding per hidden file, named by the file"
        );
        assert!(
            finding.message.contains(language.by),
            "{name}: the finding names what keeps the suite out of the run: {}",
            finding.message
        );
        assert!(
            !finding.message.contains(language.collected),
            "{name}: the suite the runner does open is not in the finding: {}",
            finding.message
        );
    }
}

#[test]
fn r5_stays_silent_on_a_tree_whose_configuration_reaches_every_test() {
    for language in LANGUAGES {
        let name = language.name;
        let repo = fixture("R5", name, "silent");
        for suite in [language.hidden, language.collected] {
            assert!(
                holds(&repo, suite),
                "{name}: the neighbour holds the same two suites, and runs both"
            );
        }

        let run = repo.weeder(&["scan"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: every suite is collected, so there is nothing to name"
        );
        assert_eq!(run.code, 0, "{name}: a scan never blocks\n{}", run.stderr);
    }
}

fn holds(repo: &Repo, path: &str) -> bool {
    repo.git(&["ls-files"]).lines().any(|held| held == path)
}

fn reported(findings: &[Finding]) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule == "R5")
        .cloned()
        .collect()
}
