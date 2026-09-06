//! `cargo xtask calibrate` on a history built for the purpose.
//!
//! The corpus the calibration file reports on is five repositories on one
//! machine, which no suite can build. What a suite can build is a history whose
//! every commit is known, and that is what these run against: the window, the
//! parents, the shape of the file, what an unclassified block counts as, and the
//! promise that the repository being read is not written to.

mod common;

use common::{suite, xtask, Bench, Repo};

/// A repository with a root commit, a clean commit, a commit that deletes a test
/// case, and a commit that edits the harness settings, which is a guardrail.
/// Two of its four commits block, and the root is not judged at all: it has no
/// parent.
fn probe() -> Repo {
    let repo = Repo::init();
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.write("tests/unit.rs", &suite(3));
    repo.write(
        ".claude/settings.json",
        "{\n  \"hooks\": {\n    \"PreToolUse\": []\n  }\n}\n",
    );
    repo.commit("the repository begins");

    repo.write(
        "src/lib.rs",
        "pub fn one() -> u32 {\n    1\n}\n\npub fn two() -> u32 {\n    2\n}\n",
    );
    repo.commit("a change that takes nothing away");

    repo.write("tests/unit.rs", &suite(2));
    repo.commit("one case fewer");

    repo.write(
        ".claude/settings.json",
        "{\n  \"hooks\": {\n    \"PreToolUse\": [],\n    \"PostToolUse\": []\n  }\n}\n",
    );
    repo.commit("the harness runs a hook after a tool too");
    repo
}

#[test]
fn judges_every_commit_that_has_a_parent_and_writes_the_verdict_first() {
    let repo = probe();
    let bench = Bench::new();

    bench.calibrate("probe", &repo, &[]).succeeded();
    let report = bench.report();

    let mut lines = report.lines();
    assert_eq!(
        lines.next(),
        Some("# calibration: weed over real history, 2026-09"),
        "the file opens with its title"
    );
    lines.next();
    let verdict = lines.next().expect("a verdict sentence");
    assert!(
        verdict.starts_with("weed does not ship as a gate:"),
        "the verdict is the first sentence, and an unread block is not a pass: {verdict}"
    );

    assert!(
        report.contains("## probe, 3 commits judged, 2 blocked"),
        "the root commit has no parent to be judged against, so three of four are judged:\n{report}"
    );
}

#[test]
fn an_unclassified_block_counts_as_a_false_positive() {
    let repo = probe();
    let bench = Bench::new();

    bench.calibrate("probe", &repo, &[]).succeeded();
    let report = bench.report();

    assert!(
        report.contains("| probe | 3 | 2 | 0 | 0 | 0 | 2 | 66.67% |"),
        "two unread blocks out of three commits is a 66.67 percent false-positive share:\n{report}"
    );
    assert!(
        report.contains("nobody has read this block yet"),
        "the table says which blocks nobody has classified:\n{report}"
    );
}

#[test]
fn a_classified_block_takes_the_class_the_ledger_gives_it() {
    let repo = probe();
    let shas: Vec<String> = repo
        .git(&["rev-list", "--reverse", "main"])
        .split_whitespace()
        .map(str::to_string)
        .collect();
    let bench = Bench::new();
    std::fs::write(
        bench.judgements(),
        format!(
            "[[commit]]\nrepo = \"probe\"\nsha = \"{}\"\nclassification = \"true-positive\"\nreasoning = \"a case really did go\"\n\n\
             [[commit]]\nrepo = \"probe\"\nsha = \"{}\"\nclassification = \"acceptable\"\nreasoning = \"a harness settings edit is what C1 watches\"\n",
            shas[2], shas[3]
        ),
    )
    .expect("the ledger should be writable");

    bench.calibrate("probe", &repo, &[]).succeeded();
    let report = bench.report();

    assert!(
        report.contains("| probe | 3 | 2 | 0 | 1 | 1 | 0 | 0.00% |"),
        "one true positive, one acceptable, no false positives:\n{report}"
    );
    assert!(
        bench
            .verdict()
            .starts_with("weed ships as a gate, pending the independent re-grade:"),
        "with nothing false left the verdict is ship, and no re-grade has read it back:\n{report}"
    );
    assert!(
        report.contains("a case really did go"),
        "the reasoning the ledger carries is what the table shows:\n{report}"
    );
    assert!(
        report.contains("No block's claim turned out to be untrue."),
        "with no false positive there is nothing under `Where weed was wrong`:\n{report}"
    );
}

#[test]
fn a_judgement_without_a_reason_is_refused() {
    let repo = probe();
    let bench = Bench::new();
    std::fs::write(
        bench.judgements(),
        "[[commit]]\nrepo = \"probe\"\nsha = \"whatever\"\nclassification = \"acceptable\"\nreasoning = \"   \"\n",
    )
    .expect("the ledger should be writable");

    let refused = bench.calibrate("probe", &repo, &[]);
    refused.failed();
    assert!(
        refused.stderr.contains("carries no reasoning"),
        "a classification nobody can review is not accepted: {}",
        refused.stderr
    );
    assert!(!bench.out().exists(), "a refused run writes no report");
}

#[test]
fn the_window_is_the_last_commits_and_nothing_older() {
    let repo = probe();
    let bench = Bench::new();

    bench
        .calibrate("probe", &repo, &["--limit", "2"])
        .succeeded();
    let report = bench.report();

    assert!(
        report.contains("## probe, 2 commits judged"),
        "a window of two judges the two newest commits:\n{report}"
    );
    assert!(
        report.contains("the harness runs a hook after a tool too"),
        "the newest commit is in the window:\n{report}"
    );
    assert!(
        !report.contains("the repository begins"),
        "the oldest commit is outside a window of two:\n{report}"
    );
}

#[test]
fn the_repository_being_read_is_left_exactly_as_it_was() {
    let repo = probe();
    let before = repo.fingerprint();
    let bench = Bench::new();

    bench.calibrate("probe", &repo, &[]).succeeded();

    assert_eq!(
        before,
        repo.fingerprint(),
        "calibration reads a repository and writes nothing to it"
    );
    assert!(
        !repo.root().join(".git/worktrees").exists(),
        "the walk adds no worktree to the repository it reads"
    );
}

#[test]
fn two_runs_over_one_history_write_the_same_bytes() {
    let repo = probe();
    let bench = Bench::new();

    bench.calibrate("probe", &repo, &[]).succeeded();
    let first = bench.report();
    bench.calibrate("probe", &repo, &[]).succeeded();
    let second = bench.report();

    assert_eq!(
        first, second,
        "the file carries no timestamp and no duration, so a diff of it is a change in judgement"
    );
}

#[test]
fn a_corpus_naming_a_source_that_is_not_there_is_refused() {
    let bench = Bench::new();
    let missing = bench.path().join("no-such-checkout");
    bench.write_corpus(
        "probe",
        &missing,
        "0123456789012345678901234567890123456789",
    );

    let refused = xtask(&[
        "calibrate",
        "--corpus",
        &bench.corpus().display().to_string(),
        "--out",
        &bench.out().display().to_string(),
        "--judgements",
        &bench.judgements().display().to_string(),
        "--first-run",
        &bench.first_run().display().to_string(),
    ]);
    refused.failed();
    assert!(
        refused.stderr.contains("`git fetch"),
        "a calibration of four repositories where five were named would read as five: {}",
        refused.stderr
    );
    assert!(!bench.out().exists(), "a refused run writes no report");
}

#[test]
fn a_pin_that_is_not_a_full_sha_is_refused() {
    let repo = probe();
    let bench = Bench::new();

    let refused = bench.calibrate_at("probe", &repo, &repo.tip()[..8], &[]);
    refused.failed();
    assert!(
        refused.stderr.contains("not a full forty-character sha"),
        "an abbreviation can come to mean a second commit: {}",
        refused.stderr
    );
}
