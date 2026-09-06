//! What bite does with the process it starts, and with the worktree it built.
//!
//! bite is the one face that runs somebody else's command, so it is the one
//! face that can be kept waiting forever. A command that will not finish is a
//! run that never happened: exit 3, one line naming the deadline it passed. And
//! whatever the verdict, clean, blocked, timed out, or a suite that never
//! passes at all, the worktree bite made is gone by the time it leaves. A judge
//! that litters checkouts across a machine is a judge nobody runs twice.

mod common;

use std::path::Path;

use common::{phased_fixture, which, Repo, Run};
use tempfile::TempDir;

/// The command the fixture's tests run under.
const SUITE: &str = "python3 -m unittest -v test_parse";

/// A command that answers to nothing and outlives any deadline weed gives it.
const HANGS: &str = "sleep 30";

/// A command that fails whatever is applied: the module it names is in no
/// commit of the fixture.
const NEVER_PASSES: &str = "python3 -m unittest -v test_absent";

fn interpreter() {
    assert!(
        which("python3").is_some(),
        "these tests run a real Python suite, and python3 is not on PATH"
    );
}

/// bite, run with a temp directory of its own, so what it leaves behind is
/// exactly what this test can see.
fn bite(repo: &Repo, temp: &TempDir, arguments: &[&str]) -> Run {
    let mut all = vec!["bite"];
    all.extend_from_slice(arguments);
    repo.weed_with(
        &all,
        &[(
            "TMPDIR",
            temp.path().to_str().expect("a utf-8 temp directory"),
        )],
    )
}

/// What the directory holds, by name. A worktree bite forgot to take away is a
/// name in here.
fn leavings(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(directory)
        .expect("the temp directory should be readable")
        .map(|entry| {
            entry
                .expect("a temp entry should be readable")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    names.sort();
    names
}

/// The worktrees git knows this repository has. One line is the checkout
/// itself; a second is a worktree bite registered and never removed.
fn worktrees(repo: &Repo) -> usize {
    repo.git(&["worktree", "list"]).lines().count()
}

fn assert_nothing_left(repo: &Repo, temp: &TempDir, after: &str) {
    assert_eq!(
        leavings(temp.path()),
        Vec::<String>::new(),
        "the worktree is gone {after}"
    );
    assert_eq!(
        worktrees(repo),
        1,
        "git is left with the checkout alone {after}"
    );
}

#[test]
fn a_test_command_that_hangs_is_exit_three_with_the_timeout_named() {
    let repo = phased_fixture("B1", "py", "silent");
    let temp = TempDir::new().expect("a temp directory for the worktree");

    let run = bite(&repo, &temp, &["--test", HANGS, "--timeout", "1"]);

    assert_eq!(run.code, 3, "a gate that could not run fails closed");
    assert_eq!(run.stderr.lines().count(), 1, "one line saying why");
    assert!(
        run.stderr.contains("1s"),
        "the deadline weed stopped waiting at is named: {}",
        run.stderr
    );
    assert!(
        run.stderr.contains(HANGS),
        "the command that would not answer is named: {}",
        run.stderr
    );
    assert_nothing_left(&repo, &temp, "after a command that would not finish");
}

#[test]
fn a_suite_that_never_passes_is_exit_three_rather_than_a_verdict() {
    interpreter();
    let repo = phased_fixture("B1", "py", "silent");
    let temp = TempDir::new().expect("a temp directory for the worktree");

    let run = bite(&repo, &temp, &["--test", NEVER_PASSES]);

    assert_eq!(
        run.code, 3,
        "a pair that is red on both sides proves nothing:\n{}{}",
        run.stdout, run.stderr
    );
    assert_eq!(run.stderr.lines().count(), 1, "one line saying why");
    assert_nothing_left(&repo, &temp, "after a suite that never passes");
}

#[test]
fn the_worktree_is_gone_whatever_the_verdict() {
    interpreter();
    for (case, code) in [("silent", 0), ("fire", 2)] {
        let repo = phased_fixture("B1", "py", case);
        let temp = TempDir::new().expect("a temp directory for the worktree");

        let run = bite(&repo, &temp, &["--test", SUITE]);

        assert_eq!(
            run.code, code,
            "{case}: the verdict this fixture carries:\n{}{}",
            run.stdout, run.stderr
        );
        assert_nothing_left(&repo, &temp, &format!("after the {case} fixture"));
    }
}

#[test]
fn a_run_that_never_starts_leaves_no_worktree_at_all() {
    let repo = phased_fixture("B1", "py", "silent");
    let temp = TempDir::new().expect("a temp directory for the worktree");

    let run = bite(
        &repo,
        &temp,
        &["--test", SUITE, "--base", "no-such-ref-anywhere"],
    );

    assert_eq!(run.code, 3);
    assert_nothing_left(&repo, &temp, "after a ref weed could not resolve");
}
