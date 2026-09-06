//! `cargo xtask calibrate` on a history built for the purpose.
//!
//! The corpus the calibration file reports on is five repositories on one
//! machine, which no suite can build. What a suite can build is a history whose
//! every commit is known, and that is what these run against: the window, the
//! parents, the shape of the file, what an unclassified block counts as, and the
//! promise that the repository being read is not written to.

mod common;

use common::{suite, workspace, xtask, Repo};

/// A repository with a root commit, a clean commit, a commit that deletes a test
/// case, and a commit that edits a guardrail file. Two of its four commits
/// block, and the root is not judged at all: it has no parent.
fn probe() -> Repo {
    let repo = Repo::init();
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.write("tests/unit.rs", &suite(3));
    repo.write(".github/workflows/ci.yml", "name: CI\non: push\n");
    repo.commit("the repository begins");

    repo.write(
        "src/lib.rs",
        "pub fn one() -> u32 {\n    1\n}\n\npub fn two() -> u32 {\n    2\n}\n",
    );
    repo.commit("a change that takes nothing away");

    repo.write("tests/unit.rs", &suite(2));
    repo.commit("one case fewer");

    repo.write(
        ".github/workflows/ci.yml",
        "name: CI\non: [push, pull_request]\n",
    );
    repo.commit("the workflow runs on pull requests too");
    repo
}

fn calibrate(
    repo: &Repo,
    out: &std::path::Path,
    judgements: &std::path::Path,
    extra: &[&str],
) -> common::Run {
    let mut arguments = vec![
        "calibrate".to_string(),
        "--repo".to_string(),
        format!("probe={}", repo.root().display()),
        "--only".to_string(),
        "probe".to_string(),
        "--out".to_string(),
        out.display().to_string(),
        "--judgements".to_string(),
        judgements.display().to_string(),
    ];
    arguments.extend(extra.iter().map(|argument| (*argument).to_string()));
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    xtask(&arguments)
}

#[test]
fn judges_every_commit_that_has_a_parent_and_writes_the_verdict_first() {
    let repo = probe();
    let run = workspace();
    let out = run.path().join("calibration.md");
    let judgements = run.path().join("judgements.toml");

    calibrate(&repo, &out, &judgements, &[]).succeeded();
    let report = std::fs::read_to_string(&out).expect("the report should be written");

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
    let run = workspace();
    let out = run.path().join("calibration.md");
    let judgements = run.path().join("judgements.toml");

    calibrate(&repo, &out, &judgements, &[]).succeeded();
    let report = std::fs::read_to_string(&out).expect("the report should be written");

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
    let run = workspace();
    let out = run.path().join("calibration.md");
    let judgements = run.path().join("judgements.toml");
    std::fs::write(
        &judgements,
        format!(
            "[[commit]]\nrepo = \"probe\"\nsha = \"{}\"\nclassification = \"true-positive\"\nreasoning = \"a case really did go\"\n\n\
             [[commit]]\nrepo = \"probe\"\nsha = \"{}\"\nclassification = \"acceptable\"\nreasoning = \"the workflow edit is what C1 watches\"\n",
            shas[2], shas[3]
        ),
    )
    .expect("the ledger should be writable");

    calibrate(&repo, &out, &judgements, &[]).succeeded();
    let report = std::fs::read_to_string(&out).expect("the report should be written");

    assert!(
        report.contains("| probe | 3 | 2 | 0 | 1 | 1 | 0 | 0.00% |"),
        "one true positive, one acceptable, no false positives:\n{report}"
    );
    assert!(
        report
            .lines()
            .nth(2)
            .is_some_and(|line| line.starts_with("weed ships as a gate:")),
        "with nothing false left, the verdict is ship:\n{report}"
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
    let run = workspace();
    let out = run.path().join("calibration.md");
    let judgements = run.path().join("judgements.toml");
    std::fs::write(
        &judgements,
        "[[commit]]\nrepo = \"probe\"\nsha = \"whatever\"\nclassification = \"acceptable\"\nreasoning = \"   \"\n",
    )
    .expect("the ledger should be writable");

    let refused = calibrate(&repo, &out, &judgements, &[]);
    refused.failed();
    assert!(
        refused.stderr.contains("carries no reasoning"),
        "a classification nobody can review is not accepted: {}",
        refused.stderr
    );
    assert!(!out.exists(), "a refused run writes no report");
}

#[test]
fn the_window_is_the_last_commits_and_nothing_older() {
    let repo = probe();
    let run = workspace();
    let out = run.path().join("calibration.md");
    let judgements = run.path().join("judgements.toml");

    calibrate(&repo, &out, &judgements, &["--limit", "2"]).succeeded();
    let report = std::fs::read_to_string(&out).expect("the report should be written");

    assert!(
        report.contains("## probe, 2 commits judged"),
        "a window of two judges the two newest commits:\n{report}"
    );
    assert!(
        report.contains("the workflow runs on pull requests too"),
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
    let run = workspace();
    let out = run.path().join("calibration.md");
    let judgements = run.path().join("judgements.toml");

    calibrate(&repo, &out, &judgements, &[]).succeeded();

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
    let run = workspace();
    let out = run.path().join("calibration.md");
    let judgements = run.path().join("judgements.toml");

    calibrate(&repo, &out, &judgements, &[]).succeeded();
    let first = std::fs::read_to_string(&out).expect("the report should be written");
    calibrate(&repo, &out, &judgements, &[]).succeeded();
    let second = std::fs::read_to_string(&out).expect("the report should be written");

    assert_eq!(
        first, second,
        "the file carries no timestamp and no duration, so a diff of it is a change in judgement"
    );
}

#[test]
fn a_corpus_naming_a_repository_that_is_not_there_is_refused() {
    let run = workspace();
    let out = run.path().join("calibration.md");
    let judgements = run.path().join("judgements.toml");
    let missing = run.path().join("no-such-checkout");

    let refused = xtask(&[
        "calibrate",
        "--repo",
        &format!("probe={}", missing.display()),
        "--only",
        "probe",
        "--out",
        &out.display().to_string(),
        "--judgements",
        &judgements.display().to_string(),
    ]);
    refused.failed();
    assert!(
        refused.stderr.contains("there is no git repository there"),
        "a calibration of four repositories where five were named would read as five: {}",
        refused.stderr
    );
}
