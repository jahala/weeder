//! S2, an error was swallowed.
//!
//! Every language has a way of catching a failure and then doing nothing with
//! it: an empty `catch`, a bare `except` over `pass`, a nil check that returns
//! nothing, an error assigned to the name that means "throw this away", a
//! fallible call turned into a default. Each one is a line where the program
//! learns something went wrong and forgets it again.
//!
//! The neighbour is the same handler doing its job: the failure logged, wrapped
//! into one the caller can read, raised on, or dropped under a comment that says
//! why dropping it is right.

mod common;

use common::{fixture, fixture_file};

/// The shapes the check names, one language at a time: the file the change
/// touches, and the line each swallowed failure is written on. The lines are
/// read back out of the fixture, so the expectation is the fixture rather than
/// a list of numbers that drifts the moment a line is added above it.
const SHAPES: [(&str, &str, &[&str]); 4] = [
    (
        "ts",
        "src/client.ts",
        &["} catch (error) {", "return drain().catch(() => {});"],
    ),
    ("py", "src/client.py", &["except Exception:", "except:"]),
    (
        "rs",
        "src/client.rs",
        &["post(payload).unwrap_or_default()", "read().ok()"],
    ),
    (
        "go",
        "client.go",
        &["if err := post(payload); err != nil {", "_ = err"],
    ),
];

#[test]
fn s2_warns_on_every_swallowed_error_shape_in_every_language() {
    for (lang, path, shapes) in SHAPES {
        let repo = fixture("S2", lang, "fire");
        let run = repo.weeder(&["check"]);

        assert_eq!(
            run.code, 0,
            "{lang}: a swallowed error is a warning, and a warning does not block\n{}",
            run.stderr
        );
        let findings = run.findings();
        let lines: Vec<u64> = findings
            .iter()
            .map(|finding| finding.line.expect("a swallowed error names its line"))
            .collect();
        assert_eq!(
            lines,
            shape_lines(lang, path, shapes),
            "{lang}: every swallowed shape is reported, and only those"
        );

        let before = fixture_file("S2", lang, "fire/before", path);
        let after = fixture_file("S2", lang, "fire/after", path);
        for finding in &findings {
            assert_eq!(finding.rule, "S2", "{lang}: the rule is S2");
            assert_eq!(finding.level, "warning", "{lang}: S2 warns");
            assert_eq!(finding.path, path, "{lang}: the finding names the file");
            let line = after
                .lines()
                .nth(finding.line.unwrap_or_default() as usize - 1)
                .expect("the reported line is a line of the file");
            assert!(
                !before.lines().any(|old| old == line),
                "{lang}: the reported line has to be one the change wrote: {line}"
            );
        }
    }
}

#[test]
fn s2_stays_silent_when_the_failure_is_logged_wrapped_raised_on_or_explained() {
    for (lang, path, _) in SHAPES {
        let repo = fixture("S2", lang, "silent");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|changed| changed.ends_with(path)),
            "{lang}: the silent neighbour must be in the diff, or the silence proves nothing"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{lang}: a failure that is read, passed on or explained is handled"
        );
        assert_eq!(run.code, 0, "{lang}: nothing found, nothing blocked");
    }
}

/// Where the fixture writes each shape, counted from one.
fn shape_lines(lang: &str, path: &str, shapes: &[&str]) -> Vec<u64> {
    let after = fixture_file("S2", lang, "fire/after", path);
    let mut lines: Vec<u64> = shapes
        .iter()
        .map(|shape| {
            after
                .lines()
                .position(|line| line.trim() == *shape)
                .map(|index| index as u64 + 1)
                .unwrap_or_else(|| panic!("{lang}: the fixture writes `{shape}` in {path}"))
        })
        .collect();
    lines.sort_unstable();
    lines
}
