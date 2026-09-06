//! S3, a debug leftover reached production code.
//!
//! Somebody wanted to see what the program was doing and left the line in. Each
//! language spells it its own way: a console call, a print, a breakpoint, a
//! macro that shouts a value back. In a library that line is output nobody
//! asked for, on a stream somebody else owns.
//!
//! The neighbour is the same line where printing is the work: a test saying
//! what it saw, the program's entry point, and a module the repository's own
//! `weeder.toml` names as a command-line face.

mod common;

use common::{fixture, fixture_file};
use tempfile::TempDir;

/// The shapes the check names, one language at a time: the production file the
/// change touches, and the line each leftover is written on.
const SHAPES: [(&str, &str, &[&str]); 4] = [
    (
        "ts",
        "src/report.ts",
        &["console.log(\"rows\", rows);", "debugger;"],
    ),
    (
        "py",
        "src/report.py",
        &["print(\"rows\", rows)", "breakpoint()", "pdb.set_trace()"],
    ),
    (
        "rs",
        "src/report.rs",
        &["println!(\"rows {rows:?}\");", "dbg!(rows);"],
    ),
    (
        "go",
        "report.go",
        &["fmt.Println(\"rows\", rows)", "inspect.Dump(rows)"],
    ),
];

/// The files the silent neighbour writes a leftover into, and why each one is
/// allowed to: a test, the entry point, and a module `weeder.toml` names.
const EXEMPT: [(&str, &[&str]); 4] = [
    ("ts", &["src/report.test.ts", "src/cli/render.ts"]),
    ("py", &["tests/test_report.py", "src/cli/render.py"]),
    (
        "rs",
        &["tests/report.rs", "src/main.rs", "src/cli/render.rs"],
    ),
    ("go", &["report_test.go", "main.go", "cli/render.go"]),
];

#[test]
fn s3_warns_on_every_debug_leftover_in_a_production_file_in_every_language() {
    for (lang, path, shapes) in SHAPES {
        let repo = fixture("S3", lang, "fire");
        let run = repo.weeder(&["check"]);

        assert_eq!(
            run.code, 0,
            "{lang}: a leftover is a warning, and a warning does not block\n{}",
            run.stderr
        );
        let findings = run.findings();
        let lines: Vec<u64> = findings
            .iter()
            .map(|finding| finding.line.expect("a leftover names its line"))
            .collect();
        assert_eq!(
            lines,
            shape_lines(lang, path, shapes),
            "{lang}: every leftover is reported, and only those"
        );
        for finding in &findings {
            assert_eq!(finding.rule, "S3", "{lang}: the rule is S3");
            assert_eq!(finding.level, "warning", "{lang}: S3 warns");
            assert_eq!(finding.path, path, "{lang}: the finding names the file");
        }
    }
}

#[test]
fn s3_stays_silent_in_a_test_in_the_entry_point_and_in_a_module_the_config_names() {
    for (lang, exempt) in EXEMPT {
        let repo = fixture("S3", lang, "silent");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        for path in exempt {
            assert!(
                changed.lines().any(|line| line == *path),
                "{lang}: {path} must be in the diff, or the silence proves nothing"
            );
        }

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{lang}: a suite, an entry point and a named command-line face all print for a living"
        );
        assert_eq!(run.code, 0, "{lang}: nothing found, nothing blocked");
    }
}

#[test]
fn the_config_is_what_exempts_a_command_line_face_and_not_its_name() {
    // Judge the same tree against a config that names no entry point, and the
    // same module is an ordinary one again: the exemption is something the
    // repository states, not a folder name weeder has heard of.
    let repo = fixture("S3", "ts", "silent");
    let elsewhere = TempDir::new().expect("a directory to keep a config in");
    let config = elsewhere.path().join("weeder.toml");
    std::fs::write(&config, "[rules]\nT1 = \"block\"\n").expect("the config should be writable");

    let run = repo.weeder(&[
        "check",
        "--config",
        config.to_str().expect("the temp path is utf-8"),
    ]);
    assert_eq!(
        run.paths(),
        vec!["src/cli/render.ts"],
        "with nothing named as a command-line face, the print is a leftover\n{}",
        run.stderr
    );
    assert!(run.findings().iter().all(|finding| finding.rule == "S3"));
}

/// Where the fixture writes each shape, counted from one.
fn shape_lines(lang: &str, path: &str, shapes: &[&str]) -> Vec<u64> {
    let after = fixture_file("S3", lang, "fire/after", path);
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
