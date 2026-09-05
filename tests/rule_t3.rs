//! T3, a skip or a focus marker was added.
//!
//! Each language spells the marker in its own grammar, and the fire fixture adds
//! every form the rule names. The lines weed must report are read off the
//! fixture here, by looking for the forms themselves, so the expectation does
//! not depend on how the detector goes about finding them.
//!
//! The neighbour writes the same tokens where they say nothing: in a comment
//! explaining that a marker came off, and in a string a case asserts on.

mod common;

use common::{fixture, fixture_file};

/// A language, the test file the markers land in, and the forms it spells them
/// with.
struct Language {
    name: &'static str,
    path: &'static str,
    forms: &'static [&'static str],
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        path: "src/format.test.ts",
        forms: &["it.skip", "it.only", "test.todo", "xit", "xdescribe"],
    },
    Language {
        name: "py",
        path: "tests/test_format.py",
        forms: &["@pytest.mark.skip", "@pytest.mark.xfail", "@unittest.skip"],
    },
    Language {
        name: "rs",
        path: "tests/format.rs",
        forms: &["#[ignore]", "#[ignore ="],
    },
    Language {
        name: "go",
        path: "format_test.go",
        forms: &["t.Skip", "t.SkipNow"],
    },
];

#[test]
fn t3_fires_at_block_level_on_every_added_skip_or_focus_form() {
    for language in LANGUAGES {
        let repo = fixture("T3", language.name, "fire");
        let run = repo.weed(&["check"]);
        let name = language.name;

        assert_eq!(run.code, 2, "{name}: a marker blocks\n{}", run.stderr);
        let findings = run.findings();
        let expected = marker_lines(&language, "fire/after");
        assert!(
            expected.len() >= language.forms.len(),
            "{name}: the fixture must carry every form the rule names"
        );
        assert_eq!(
            findings
                .iter()
                .map(|finding| finding.line.expect("a marker finding names its line"))
                .collect::<Vec<u64>>(),
            expected,
            "{name}: every marker line is reported, and only those"
        );
        for finding in &findings {
            assert_eq!(finding.rule, "T3", "{name}: the rule is T3");
            assert_eq!(finding.level, "error", "{name}: T3 blocks");
            assert_eq!(finding.path, language.path, "{name}: the file is named");
        }
    }
}

#[test]
fn t3_stays_silent_when_the_same_token_sits_in_a_string_or_a_comment() {
    for language in LANGUAGES {
        let repo = fixture("T3", language.name, "silent");
        let name = language.name;

        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path == language.path),
            "{name}: the neighbour must be in the diff, or the silence proves nothing"
        );
        assert!(
            !marker_lines(&language, "silent/after").is_empty(),
            "{name}: the neighbour has to write the tokens down, or it is not a neighbour"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a marker a file only talks about is not a marker"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// Where the forms the rule names are written in a fixture's test file, counted
/// from one and each line named once however many forms it carries.
fn marker_lines(language: &Language, case: &str) -> Vec<u64> {
    let source = fixture_file("T3", language.name, case, language.path);
    let mut lines: Vec<u64> = source
        .lines()
        .enumerate()
        .filter(|(_, line)| language.forms.iter().any(|form| line.contains(form)))
        .map(|(index, _)| index as u64 + 1)
        .collect();
    lines.sort_unstable();
    lines.dedup();
    lines
}
