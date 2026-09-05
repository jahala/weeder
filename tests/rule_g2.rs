//! G2, a large or a binary file was added.
//!
//! A repository is text somebody can read a diff of. A blob nobody can review
//! arrives once and is carried by every clone from then on, and the usual way
//! one gets in is that a build wrote it and an agent staged everything.
//!
//! These files are built here rather than kept in `fixtures/`: a mebibyte of
//! filler and a run of bytes that are not text are exactly the two things this
//! repository should not be carrying around to prove a point.

mod common;

use common::Repo;

/// The size the rule draws the line at.
const MEBIBYTE: usize = 1024 * 1024;

/// A file of readable text, larger than the threshold.
fn big_text() -> String {
    let row = "the quick brown fox jumps over the lazy dog\n";
    row.repeat(MEBIBYTE / row.len() + 2)
}

/// A run of bytes no diff can show: a header, and a NUL, which is git's own
/// test for a blob that is not text.
fn binary() -> String {
    let mut bytes = String::from("WEEDFIXTURE\u{0}");
    for step in 0..512 {
        bytes.push(char::from(u8::try_from(step % 256).unwrap_or_default()));
    }
    bytes
}

#[test]
fn g2_warns_on_an_added_file_above_a_mebibyte_and_on_an_added_binary() {
    let repo = Repo::init();
    repo.write("src/index.ts", "export const ready = true;\n");
    repo.commit("the state the change starts from");

    let filler = big_text();
    assert!(
        filler.len() > MEBIBYTE,
        "the fixture is above the threshold"
    );
    repo.write("assets/corpus.txt", &filler);
    repo.write("assets/mark.bin", &binary());
    repo.stage_all();

    let run = repo.weed(&["check"]);
    assert_eq!(
        run.code, 0,
        "a large file is a warning, and a warning does not block\n{}",
        run.stderr
    );
    let findings = run.findings();
    assert_eq!(
        run.paths(),
        vec!["assets/corpus.txt", "assets/mark.bin"],
        "both blobs are named, and nothing else is"
    );
    for finding in &findings {
        assert_eq!(finding.rule, "G2", "the rule is G2");
        assert_eq!(finding.level, "warning", "G2 warns");
    }
    let large = message(&findings, "assets/corpus.txt");
    let blob = message(&findings, "assets/mark.bin");
    assert_ne!(
        large, blob,
        "a file too big to read and a file that is not text are two different findings"
    );
}

#[test]
fn g2_stays_silent_on_a_small_text_file() {
    let repo = Repo::init();
    repo.write("src/index.ts", "export const ready = true;\n");
    repo.commit("the state the change starts from");

    repo.write(
        "src/parser.ts",
        "export const parse = (s: string) => s.trim();\n",
    );
    repo.stage_all();

    let run = repo.weed(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "a file somebody can read the diff of is an ordinary file\n{}",
        run.stderr
    );
    assert_eq!(run.code, 0, "nothing found, nothing blocked");
}

#[test]
fn g2_stays_silent_on_a_tracked_binary_that_only_changes() {
    let repo = Repo::init();
    repo.write("assets/mark.bin", &binary());
    repo.commit("the blob was already here, and somebody already said yes to it");

    let mut changed = binary();
    changed.push('\u{7}');
    repo.write("assets/mark.bin", &changed);
    repo.stage_all();
    assert!(
        repo.git(&["diff", "--cached", "--name-only"])
            .lines()
            .any(|line| line == "assets/mark.bin"),
        "the blob must be in the diff, or the silence proves nothing"
    );

    let run = repo.weed(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "the decision to carry this blob was made before this change\n{}",
        run.stderr
    );
    assert_eq!(run.code, 0, "nothing found, nothing blocked");
}

/// What the run said about one path.
fn message(findings: &[common::Finding], path: &str) -> String {
    findings
        .iter()
        .find(|finding| finding.path == path)
        .map(|finding| finding.message.clone())
        .unwrap_or_else(|| panic!("{path} should carry a finding"))
}
