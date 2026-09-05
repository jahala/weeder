//! C2, an ignore file was broadened over source or tests.
//!
//! An ignore file decides what the tools and the reviewer are shown. A pattern
//! added there that covers the repository's own source or its suite takes work
//! out of sight without touching a line of it, and the next diff looks clean
//! because half of it is no longer being read.
//!
//! The neighbour is what an ignore file is for: build output, installed
//! packages, coverage, the minified copy of something that already has a source.

mod common;

use common::{fixture, fixture_file, IGNORE_FILE_IN_FIXTURE};

/// The ignore file the fixture changes.
const IGNORE: &str = ".gitignore";

/// The patterns the fire fixture adds, each of which hides something the
/// repository is made of.
const BROADENING: [&str; 2] = ["src/", "*.test.ts"];

#[test]
fn c2_warns_on_every_added_pattern_that_covers_source_or_tests() {
    let repo = fixture("C2", "paths", "fire");
    let run = repo.weed(&["check"]);

    assert_eq!(
        run.code, 0,
        "a broadened ignore is a warning, and a warning does not block\n{}",
        run.stderr
    );
    let findings = run.findings();
    let lines: Vec<u64> = findings
        .iter()
        .map(|finding| finding.line.expect("a pattern finding names its line"))
        .collect();
    assert_eq!(
        lines,
        pattern_lines(&BROADENING),
        "every broadening pattern is reported, and only those"
    );
    for finding in &findings {
        assert_eq!(finding.rule, "C2", "the rule is C2");
        assert_eq!(finding.level, "warning", "C2 warns");
        assert_eq!(finding.path, IGNORE, "the finding names the ignore file");
    }
    for pattern in BROADENING {
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains(pattern)),
            "the message quotes the pattern that was added: {pattern}"
        );
    }
}

#[test]
fn c2_stays_silent_on_build_output() {
    let repo = fixture("C2", "paths", "silent");
    let changed = repo.git(&["diff", "HEAD", "--name-only"]);
    assert!(
        changed.lines().any(|line| line == IGNORE),
        "the neighbour must be in the diff, or the silence proves nothing"
    );

    let run = repo.weed(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "output a build wrote is what an ignore file is for"
    );
    assert_eq!(run.code, 0, "nothing found, nothing blocked");
}

/// Where the fixture writes each pattern, counted from one.
fn pattern_lines(patterns: &[&str]) -> Vec<u64> {
    let after = fixture_file("C2", "paths", "fire/after", IGNORE_FILE_IN_FIXTURE);
    let mut lines: Vec<u64> = patterns
        .iter()
        .map(|pattern| {
            after
                .lines()
                .position(|line| line.trim() == *pattern)
                .map(|index| index as u64 + 1)
                .unwrap_or_else(|| panic!("the fixture writes `{pattern}` in {IGNORE}"))
        })
        .collect();
    lines.sort_unstable();
    lines
}
