//! T1, a test was deleted.
//!
//! Two shapes, in four languages. A test file removed takes every case it held;
//! a test file that stayed and declares fewer cases lost the difference. The
//! neighbour does the two honest things that look the same from a distance, a
//! case renamed where it stands, and a case added, and weed says nothing.
//!
//! Every number here is counted off the fixture by this file's own reading of
//! what a case looks like, so the expectation is written independently of the
//! detector that has to meet it.

mod common;

use common::{fixture, fixture_file};

/// A language, the test file the change deletes, and the test file it thins.
struct Language {
    name: &'static str,
    deleted: &'static str,
    thinned: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        deleted: "src/parser.test.ts",
        thinned: "src/format.test.ts",
    },
    Language {
        name: "py",
        deleted: "tests/test_parser.py",
        thinned: "tests/test_format.py",
    },
    Language {
        name: "rs",
        deleted: "tests/parser.rs",
        thinned: "tests/format.rs",
    },
    Language {
        name: "go",
        deleted: "parser_test.go",
        thinned: "format_test.go",
    },
];

#[test]
fn t1_fires_at_block_level_on_a_deleted_test_file_and_a_thinned_one() {
    for language in LANGUAGES {
        let repo = fixture("T1", language.name, "fire");
        let run = repo.weed(&["check"]);
        let name = language.name;

        assert_eq!(run.code, 2, "{name}: a deleted test blocks\n{}", run.stderr);
        let (findings, others): (Vec<common::Finding>, Vec<common::Finding>) = run
            .findings()
            .into_iter()
            .partition(|finding| finding.rule == "T1");
        assert_eq!(
            findings.len(),
            2,
            "{name}: the file that went and the file that shrank, and nothing else: {findings:#?}"
        );
        for finding in &findings {
            assert_eq!(finding.level, "error", "{name}: T1 blocks");
        }
        // A case that goes takes its assertions with it, so T2 reads the file
        // that shrank and says its own thing about it. Nothing else may.
        assert!(
            others
                .iter()
                .all(|finding| finding.rule == "T2" && finding.path == language.thinned),
            "{name}: the only other reading of this change is the assertions that went with the case: {others:#?}"
        );
        assert_eq!(
            sorted_paths(&findings),
            sorted(vec![language.deleted, language.thinned]),
            "{name}: both files are named"
        );

        let gone = finding_on(&findings, language.deleted);
        let held = cases(name, "fire/before", language.deleted);
        assert!(
            gone.message.contains(&held.to_string()),
            "{name}: the finding says how many cases went with the file: {}",
            gone.message
        );

        let thinned = finding_on(&findings, language.thinned);
        let before = cases(name, "fire/before", language.thinned);
        let after = cases(name, "fire/after", language.thinned);
        assert!(
            after < before,
            "{name}: the fixture has to lose a case, or the check proves nothing"
        );
        assert!(
            thinned.message.contains(&before.to_string())
                && thinned.message.contains(&after.to_string()),
            "{name}: the finding says what the count was and what it is: {}",
            thinned.message
        );
    }
}

#[test]
fn t1_stays_silent_when_a_case_is_renamed_in_place_or_a_new_one_is_added() {
    for language in LANGUAGES {
        let repo = fixture("T1", language.name, "silent");
        let name = language.name;

        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path == language.thinned),
            "{name}: the neighbour must be in the diff, or the silence proves nothing"
        );
        let before = cases(name, "silent/before", language.thinned);
        let after = cases(name, "silent/after", language.thinned);
        assert!(
            after > before,
            "{name}: the neighbour renames one case and adds another, so the count rises"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a rename in place and a new case take nothing away"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

fn finding_on(findings: &[common::Finding], path: &str) -> common::Finding {
    findings
        .iter()
        .find(|finding| finding.path == path)
        .unwrap_or_else(|| panic!("a finding on {path}, among {findings:#?}"))
        .clone()
}

/// The files a set of findings named, sorted and without repeats.
fn sorted_paths(findings: &[common::Finding]) -> Vec<String> {
    let mut paths: Vec<String> = findings
        .iter()
        .map(|finding| finding.path.clone())
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

fn sorted(mut paths: Vec<&str>) -> Vec<String> {
    paths.sort_unstable();
    paths.into_iter().map(ToString::to_string).collect()
}

/// How many cases a fixture's test file declares, counted the way each language
/// writes one down.
fn cases(lang: &str, case: &str, path: &str) -> usize {
    let source = fixture_file("T1", lang, case, path);
    source
        .lines()
        .map(str::trim)
        .filter(|line| match lang {
            "ts" => line.starts_with("it("),
            "py" => line.starts_with("def test"),
            "rs" => *line == "#[test]",
            "go" => line.starts_with("func Test"),
            other => panic!("no case shape is written down for {other}"),
        })
        .count()
}
