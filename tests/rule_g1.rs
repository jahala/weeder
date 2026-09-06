//! G1, a conflict marker was committed.
//!
//! Every case runs the built binary on a real repository built from
//! `fixtures/adversarial/G1/`, and reads the SARIF it wrote.

mod common;

use common::{fixture, fixture_file, ours, separator, theirs};

/// The languages the rule is proven in, and the file the conflict lands in.
const LANGUAGES: [(&str, &str); 4] = [
    ("ts", "src/parser.ts"),
    ("py", "src/parser.py"),
    ("rs", "src/parser.rs"),
    ("go", "parser.go"),
];

/// The markers the check names, built here from the harness rather than read
/// from the detector, so the expectation is written independently of the code
/// that has to meet it.
fn markers() -> [String; 3] {
    [ours(""), separator(), theirs("")]
}

#[test]
fn g1_fires_at_block_level_on_every_conflict_marker_in_every_language() {
    for (lang, path) in LANGUAGES {
        let repo = fixture("G1", lang, "fire");
        let run = repo.weeder(&["check"]);

        assert_eq!(
            run.code, 2,
            "{lang}: a conflict marker blocks\n{}",
            run.stderr
        );
        let findings = run.findings();
        let lines: Vec<u64> = findings
            .iter()
            .map(|finding| finding.line.expect("a marker finding names its line"))
            .collect();
        assert_eq!(
            lines,
            marker_lines(lang, path),
            "{lang}: every marker line is reported, and only those"
        );

        for finding in &findings {
            assert_eq!(finding.rule, "G1", "{lang}: the rule is G1");
            assert_eq!(finding.level, "error", "{lang}: G1 blocks");
            assert_eq!(finding.path, path, "{lang}: the finding names the file");
            assert!(
                markers()
                    .iter()
                    .any(|marker| finding.message.contains(marker)),
                "{lang}: the message names the marker it found: {}",
                finding.message
            );
        }
    }
}

#[test]
fn g1_stays_silent_on_a_separator_in_a_string_and_a_markdown_rule() {
    for (lang, _) in LANGUAGES {
        let repo = fixture("G1", lang, "silent");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path.ends_with(".md")),
            "{lang}: the silent neighbour must be in the diff, or the silence proves nothing"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{lang}: a separator with no conflict around it is punctuation"
        );
        assert_eq!(run.code, 0, "{lang}: nothing found, nothing blocked");
    }
}

#[test]
fn the_pending_commit_message_never_reaches_the_tree() {
    let repo = fixture("G1", "ts", "fire");
    assert!(
        !repo.root().join(".weeder-commit").exists(),
        "the fixture's commit message is not a file of the change"
    );
    let staged = repo.git(&["diff", "--cached", "--name-only"]);
    assert!(
        !staged.lines().any(|path| path == ".weeder-commit"),
        "the fixture's commit message is never staged"
    );
}

/// Where the markers the check names sit in the fixture's file, counted from one.
fn marker_lines(lang: &str, path: &str) -> Vec<u64> {
    let markers = markers();
    fixture_file("G1", lang, "fire/after", path)
        .lines()
        .enumerate()
        .filter(|(_, line)| markers.iter().any(|marker| line.starts_with(marker)))
        .map(|(index, _)| index as u64 + 1)
        .collect()
}
