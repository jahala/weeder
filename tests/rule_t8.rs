//! T8, a configuration line took a test out of the run.
//!
//! T7 reads the name a file was renamed to. This reads the settings around the
//! file. Every fire fixture adds one line to what the runner reads, and that
//! line stops an existing suite from being collected: an ignore list, a path
//! pattern, a gate written at the top of the suite itself. Nothing else moves,
//! so the diff is that line and only that line.
//!
//! The neighbours add the same kind of line and take no test out of the run:
//! one names a file the tree does not hold, one names a directory of material a
//! test reads, and one gates a file the runner never collected. A line that
//! hides nothing is a line weeder says nothing about.

mod common;

use common::{fixture, fixture_file, Finding, Repo};

/// A language, the suite a line takes out of the run, the file that line is
/// written in, and what the line is called there.
struct Language {
    name: &'static str,
    hidden: &'static str,
    by: &'static str,
    setting: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        hidden: "src/format.test.ts",
        by: "jest.config.js",
        setting: "testPathIgnorePatterns",
    },
    Language {
        name: "py",
        hidden: "tests/test_format.py",
        by: "tests/conftest.py",
        setting: "collect_ignore",
    },
    // A crate-level gate hides the file it opens, so the line and the suite it
    // takes out of the run are the same file.
    Language {
        name: "rs",
        hidden: "tests/format.rs",
        by: "tests/format.rs",
        setting: "cfg",
    },
    Language {
        name: "go",
        hidden: "format_test.go",
        by: "format_test.go",
        setting: "go:build",
    },
];

/// The neighbour of each fire fixture: the file it edits, and what makes the
/// same shape honest there.
const NEIGHBOURS: [(&str, &str); 4] = [
    ("ts", "jest.config.js"),
    ("py", "tests/conftest.py"),
    ("rs", "tests/support/helpers.rs"),
    ("go", "fixtures/sample_test.go"),
];

#[test]
fn t8_fires_at_block_level_on_a_line_that_takes_an_existing_test_out_of_the_run() {
    for language in LANGUAGES {
        let name = language.name;
        let repo = fixture("T8", name, "fire");
        assert_eq!(
            changed(&repo),
            vec![language.by.to_string()],
            "{name}: the fixture changes the one file the line is written in"
        );
        assert!(
            repo.git(&["ls-files"])
                .lines()
                .any(|path| path == language.hidden),
            "{name}: the suite the line hides has to still be in the tree"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.code, 2,
            "{name}: a test taken out of the run blocks\n{}",
            run.stderr
        );

        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one suite, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "error", "{name}: T8 blocks");
        assert_eq!(
            finding.path, language.hidden,
            "{name}: the finding is named by the file it hides"
        );
        let at = format!("{}:{}", language.by, line_of("fire", &language));
        assert!(
            finding.message.contains(language.hidden)
                && finding.message.contains(&at)
                && finding.message.contains(language.setting),
            "{name}: the finding names the suite, the line and the setting: {}",
            finding.message
        );
    }
}

#[test]
fn t8_stays_silent_where_the_same_line_takes_no_test_out_of_the_run() {
    for (name, edited) in NEIGHBOURS {
        let repo = fixture("T8", name, "silent");
        assert_eq!(
            changed(&repo),
            vec![edited.to_string()],
            "{name}: the neighbour has to edit something, or its silence proves nothing"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a line that hides no suite is a line weeder says nothing about"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t8_warns_where_the_line_narrows_a_pattern_and_the_hidden_set_is_worked_out() {
    let repo = fixture("T8", "py", "narrow");
    let run = repo.weeder(&["check"]);

    let findings = reported(&run.findings());
    assert_eq!(
        findings.len(),
        1,
        "one suite is left outside the narrowed pattern: {findings:?}"
    );
    let finding = &findings[0];
    assert_eq!(
        finding.level, "warning",
        "the line names no file, so the reader is told rather than stopped: {}",
        finding.message
    );
    assert_eq!(
        finding.path, "tests/e2e/test_flow.py",
        "the finding is named by the suite the pattern no longer reaches"
    );
    assert!(
        finding.message.contains("pytest.ini:3") && finding.message.contains("testpaths"),
        "the finding names the line that narrowed the run: {}",
        finding.message
    );
    assert_eq!(
        run.code, 0,
        "a warning does not stop a change\n{}",
        run.stderr
    );
}

#[test]
fn t8_warns_where_the_line_takes_one_case_out_and_leaves_the_file_running() {
    let repo = fixture("T8", "py", "deselect");
    let run = repo.weeder(&["check"]);

    let findings = reported(&run.findings());
    assert_eq!(findings.len(), 1, "one file is named: {findings:?}");
    let finding = &findings[0];
    assert_eq!(
        finding.level, "warning",
        "the file still runs, so the reader is told rather than stopped: {}",
        finding.message
    );
    assert_eq!(finding.path, "tests/test_format.py");
    assert!(
        finding.message.contains("pytest.ini:2") && finding.message.contains("addopts"),
        "the finding names the line that took the case out: {}",
        finding.message
    );
    assert_eq!(
        run.code, 0,
        "a warning does not stop a change\n{}",
        run.stderr
    );
}

#[test]
fn t8_warns_where_the_runner_is_told_to_stop_finding_suites_for_itself() {
    let repo = fixture("T8", "rs", "autotests");
    let run = repo.weeder(&["check"]);

    let findings = reported(&run.findings());
    assert_eq!(
        findings.len(),
        1,
        "the crate declares no target of its own, so its one suite is out: {findings:?}"
    );
    let finding = &findings[0];
    assert_eq!(
        finding.level, "warning",
        "the line names no file, so the reader is told rather than stopped: {}",
        finding.message
    );
    assert_eq!(finding.path, "tests/format.rs");
    assert!(
        finding.message.contains("Cargo.toml:5") && finding.message.contains("autotests"),
        "the finding names the line that stopped the run finding it: {}",
        finding.message
    );
    assert_eq!(
        run.code, 0,
        "a warning does not stop a change\n{}",
        run.stderr
    );
}

#[test]
fn a_computed_ignore_list_is_reported_unreadable_and_refused_under_strict() {
    let repo = fixture("T8", "py", "computed");

    let run = repo.weeder(&["check"]);
    assert_eq!(
        reported(&run.findings()),
        Vec::new(),
        "weeder cannot read what the list will hold, so it claims nothing about it"
    );
    assert!(
        run.stderr.contains("tests/conftest.py:6") && run.stderr.contains("collect_ignore"),
        "the run says which setting it could not read: {}",
        run.stderr
    );

    let strict = repo.weeder(&["check", "--strict"]);
    assert_eq!(
        strict.code, 3,
        "under --strict a run that cannot read the setting is a run that did not happen\n{}",
        strict.stderr
    );
    assert!(
        strict.stderr.contains("tests/conftest.py:6"),
        "the refusal says why: {}",
        strict.stderr
    );
}

/// The 1-based line the setting is written on in the fixture's after state, so
/// the test reads the place from the fixture rather than from a number typed
/// beside it.
fn line_of(case: &str, language: &Language) -> u32 {
    let text = fixture_file("T8", language.name, case, &format!("after/{}", language.by));
    let at = text
        .lines()
        .position(|line| line.contains(language.setting))
        .unwrap_or_else(|| {
            panic!(
                "{}: after/{} should carry the {} the case is about",
                language.name, language.by, language.setting
            )
        });
    at as u32 + 1
}

/// The paths the fixture's change touched.
fn changed(repo: &Repo) -> Vec<String> {
    repo.git(&["diff", "HEAD", "--name-only"])
        .lines()
        .map(ToString::to_string)
        .collect()
}

fn reported(findings: &[Finding]) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule == "T8")
        .cloned()
        .collect()
}
