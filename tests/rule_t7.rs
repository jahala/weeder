//! T7, a rename took a test out of the runner.
//!
//! The file fixtures rename a suite to a name its runner does not collect, in
//! all four languages, and the neighbour renames the same file to another name
//! the runner does collect. Nothing else moves, so the diff is a rename and
//! only a rename.
//!
//! The case fixtures are for the two languages whose runners decide what a case
//! is from its name. Where a case is marked instead, an attribute above it, a
//! title handed to a call, a rename collects the test as before, and there is
//! nothing for this rule to find.

mod common;

use common::{fixture, Finding, Repo};

/// A language, the file its suite starts in, and where a rename takes it: out
/// of what the runner collects, and to another name inside it.
struct Language {
    name: &'static str,
    from: &'static str,
    outside: &'static str,
    inside: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        from: "src/format.test.ts",
        outside: "src/format.helpers.ts",
        inside: "src/formatter.test.ts",
    },
    Language {
        name: "py",
        from: "tests/test_format.py",
        outside: "tests/format_helpers.py",
        inside: "tests/test_formatter.py",
    },
    Language {
        name: "rs",
        from: "tests/format.rs",
        outside: "tests/support/format.rs",
        inside: "tests/formatting.rs",
    },
    Language {
        name: "go",
        from: "format_test.go",
        outside: "format_helpers.go",
        inside: "formatter_test.go",
    },
];

/// The languages whose runners collect a case by the name it is declared under,
/// the file the case lives in, and the two names the fixtures rename it to.
struct CaseLanguage {
    name: &'static str,
    path: &'static str,
    was: &'static str,
    outside: &'static str,
}

const CASE_LANGUAGES: [CaseLanguage; 2] = [
    CaseLanguage {
        name: "py",
        path: "tests/test_format.py",
        was: "test_pads_to_the_width",
        outside: "pads_to_the_width",
    },
    CaseLanguage {
        name: "go",
        path: "format_test.go",
        was: "TestPadsToTheWidth",
        outside: "checkPadsToTheWidth",
    },
];

#[test]
fn t7_fires_at_block_level_on_a_file_renamed_out_of_the_runner() {
    for language in LANGUAGES {
        let repo = fixture("T7", language.name, "fire");
        let name = language.name;
        assert_eq!(
            renamed(&repo),
            vec![(language.from.to_string(), language.outside.to_string())],
            "{name}: the fixture is a rename and nothing else"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.code, 2,
            "{name}: an uncollected test blocks\n{}",
            run.stderr
        );

        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one file, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "error", "{name}: T7 blocks");
        assert_eq!(
            finding.path, language.outside,
            "{name}: the finding lands where the file went"
        );
        assert!(
            finding.message.contains(language.from) && finding.message.contains(language.outside),
            "{name}: the finding names both ends of the rename: {}",
            finding.message
        );
    }
}

#[test]
fn t7_stays_silent_on_a_rename_that_stays_inside_the_convention() {
    for language in LANGUAGES {
        let repo = fixture("T7", language.name, "silent");
        let name = language.name;
        assert_eq!(
            renamed(&repo),
            vec![(language.from.to_string(), language.inside.to_string())],
            "{name}: the neighbour renames the same file to a name the runner still collects"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a suite the runner still opens is a suite that still runs"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t7_fires_at_block_level_on_a_case_renamed_out_of_the_runner() {
    for language in CASE_LANGUAGES {
        let repo = fixture("T7", language.name, "fire-case");
        let name = language.name;

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.code, 2,
            "{name}: an uncollected case blocks\n{}",
            run.stderr
        );

        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one case, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "error", "{name}: T7 blocks");
        assert_eq!(finding.path, language.path, "{name}: the file is named");
        assert!(
            finding.message.contains(language.was) && finding.message.contains(language.outside),
            "{name}: the finding names both ends of the rename: {}",
            finding.message
        );
    }
}

#[test]
fn t7_stays_silent_on_a_case_rename_that_stays_inside_the_convention() {
    for language in CASE_LANGUAGES {
        let repo = fixture("T7", language.name, "silent-case");
        let name = language.name;

        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path == language.path),
            "{name}: the neighbour must be in the diff, or the silence proves nothing"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a case the runner still calls is a case that still runs"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// The renames git read out of the fixture's change, each as the path that went
/// and the path that arrived.
fn renamed(repo: &Repo) -> Vec<(String, String)> {
    repo.git(&["diff", "HEAD", "--find-renames", "--name-status"])
        .lines()
        .filter_map(|line| {
            let mut cells = line.split('\t');
            let status = cells.next()?;
            if !status.starts_with('R') {
                return None;
            }
            Some((cells.next()?.to_string(), cells.next()?.to_string()))
        })
        .collect()
}

fn reported(findings: &[Finding]) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule == "T7")
        .cloned()
        .collect()
}
