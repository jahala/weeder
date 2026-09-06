//! `weed bite`, the test held to its own failure.
//!
//! The fixture is a repository whose history carries the three commits a phased
//! node leaves: the state the work starts from, the tests alone, the
//! implementation. bite checks the test commit out on the base in a worktree of
//! its own and runs the command the caller named. A command that already passes
//! there is a test that proves nothing about the change beneath it, and that is
//! B1, at block level, naming the cases the test commit added.
//!
//! Nothing here is simulated: the fixture is a Python project, the command is
//! Python's own runner, and the pass or the failure weed reads is the one the
//! interpreter decided. B1 has one language rather than four because it reads no
//! language-specific shape of its own, it runs whatever command the caller
//! named; the case names it prints come from the reader, which `tests/reader_langs.rs`
//! holds to all four.

mod common;

use common::{phased_fixture, which, Repo, Run};

/// The command the fixture's tests run under. `unittest` ships with the
/// interpreter, so the fixture is a project that really runs.
const SUITE: &str = "python3 -m unittest -v test_parse";

/// The interpreter the fixture needs. A machine without it cannot prove
/// anything about a test command, so the test says so rather than passing.
fn interpreter() {
    assert!(
        which("python3").is_some(),
        "these tests run a real Python suite, and python3 is not on PATH"
    );
}

fn bite(repo: &Repo, arguments: &[&str]) -> Run {
    let mut all = vec!["bite", "--test", SUITE];
    all.extend_from_slice(arguments);
    repo.weed(&all)
}

/// How many sentences a message carries. A finding says what was found, why it
/// matters and what to do next, in that order, so three is the number.
fn sentences(text: &str) -> usize {
    text.split(". ").count()
}

#[test]
fn a_test_that_fails_alone_and_passes_with_the_change_is_clean() {
    interpreter();
    let repo = phased_fixture("B1", "py", "silent");

    let run = bite(&repo, &[]);

    assert_eq!(
        run.code, 0,
        "the suite fails on the test commit alone and passes with the implementation:\n{}{}",
        run.stdout, run.stderr
    );
    assert_eq!(run.findings(), Vec::new(), "a change that earns its test");
    assert_eq!(run.stderr, "");
}

#[test]
fn a_test_that_passes_without_the_change_is_b1_at_block_level_naming_it() {
    interpreter();
    let repo = phased_fixture("B1", "py", "fire");

    let run = bite(&repo, &[]);

    assert_eq!(
        run.code, 2,
        "a test that passes without its change blocks:\n{}{}",
        run.stdout, run.stderr
    );
    let findings = run.findings();
    assert!(!findings.is_empty(), "the fire fixture has to be reported");
    for finding in &findings {
        assert_eq!(finding.rule, "B1");
        assert_eq!(finding.level, "error", "B1 blocks");
        assert_eq!(finding.path, "test_parse.py");
        assert!(finding.line.is_some(), "a case has a line to point at");
        assert_eq!(
            sentences(&finding.message),
            3,
            "what, why and next: {}",
            finding.message
        );
    }

    let said = findings
        .iter()
        .map(|finding| finding.message.clone())
        .collect::<Vec<String>>()
        .join(" ");
    for case in [
        "test_keeps_an_empty_field",
        "test_keeps_a_single_field_whole",
    ] {
        assert!(
            said.contains(case),
            "the tests that passed are named, and {case} is not in: {said}"
        );
    }
    assert!(
        !said.contains("test_splits_on_commas"),
        "the base already ran that case, so the test commit did not add it: {said}"
    );
}

#[test]
fn the_commits_the_caller_names_are_the_ones_judged() {
    interpreter();
    let repo = phased_fixture("B1", "py", "silent");
    // Work landed on top of the pair, the way a branch carries more than one
    // node. With the pair named, the tip has nothing to say about the verdict.
    repo.write("README.md", "The importer reads the supplier feed.\n");
    repo.commit("write down what the importer is for");

    let run = bite(
        &repo,
        &[
            "--base",
            "HEAD~3",
            "--test-commit",
            "HEAD~2",
            "--impl-commit",
            "HEAD~1",
        ],
    );

    assert_eq!(
        run.code, 0,
        "the named pair is judged, not the tip:\n{}{}",
        run.stdout, run.stderr
    );
    assert_eq!(run.findings(), Vec::new());
}

#[test]
fn a_history_with_no_phase_pair_in_it_is_a_run_that_never_happened() {
    interpreter();
    let repo = Repo::init();
    repo.write("parse.py", "def parse(row):\n    return row.split(\",\")\n");
    repo.commit("the whole thing in one commit");

    let run = bite(&repo, &[]);

    assert_eq!(run.code, 3, "a gate that could not run fails closed");
    assert_eq!(run.stderr.lines().count(), 1, "one line saying why");
    assert!(
        run.stderr.contains("HEAD~2"),
        "the ref weed could not resolve is named: {}",
        run.stderr
    );
}

#[test]
fn the_log_declares_b1_and_the_exit_code_it_left_with() {
    interpreter();
    let repo = phased_fixture("B1", "py", "fire");

    let run = bite(&repo, &[]);
    let log = run.log();

    let rules = log["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("a run declares the rules it could report");
    assert_eq!(
        rules
            .iter()
            .map(|rule| rule["id"].clone())
            .collect::<Vec<_>>(),
        vec![serde_json::json!("B1")],
        "bite reports one rule, and the log says so"
    );
    assert_eq!(
        log["runs"][0]["invocations"][0]["exitCode"],
        serde_json::json!(2)
    );
    assert_eq!(
        log["runs"][0]["invocations"][0]["executionSuccessful"],
        serde_json::json!(true),
        "bite ran and reached a verdict"
    );
}
