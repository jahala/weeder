//! The exit codes `weed check` leaves with, and what it says when it cannot run.
//!
//! 0 is clean or warnings only, 2 is at least one block-level result, and 3 is a
//! run that never reached a judgement. A gate that could not run must never look
//! like a gate that passed, so every exit 3 carries one line on stderr naming the
//! cause.

mod common;

use common::{conflicted_parser, weed_in, Repo};
use tempfile::TempDir;

#[test]
fn a_clean_diff_exits_zero() {
    let repo = Repo::init();
    repo.write(
        "src/parser.ts",
        "export const parse = (input: string) => input;\n",
    );
    repo.stage_all();

    let run = repo.weed(&["check"]);
    assert_eq!(run.findings(), Vec::new());
    assert_eq!(run.code, 0);
    assert_eq!(run.stderr, "", "a clean run says nothing");
}

#[test]
fn warnings_alone_exit_zero() {
    let repo = Repo::init();
    // The config file is itself a guardrail, so writing one is a C1 finding.
    // Both rules warn here, which is what makes this a run with warnings and
    // nothing above them.
    repo.write("weed.toml", "[rules]\nG1 = \"warn\"\nC1 = \"warn\"\n");
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    let run = repo.weed(&["check"]);
    let findings = run.findings();
    assert!(!findings.is_empty(), "the rule still reports");
    assert!(
        findings.iter().all(|finding| finding.level == "warning"),
        "a rule configured to warn warns"
    );
    assert_eq!(
        run.code, 0,
        "warnings are for the human at the pull request"
    );
}

#[test]
fn one_block_level_result_exits_two() {
    let repo = Repo::init();
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    let run = repo.weed(&["check"]);
    assert!(run
        .findings()
        .iter()
        .any(|finding| finding.level == "error"));
    assert_eq!(run.code, 2);
}

#[test]
fn outside_a_git_repository_weed_exits_three() {
    let elsewhere = TempDir::new().expect("a directory that is not a repository");
    let run = weed_in(elsewhere.path(), &["check"]);

    assert_eq!(run.code, 3);
    assert_eq!(run.stderr_lines().len(), 1, "one line, naming the cause");
    assert!(
        run.stderr.contains("git repository"),
        "the line says what was missing: {}",
        run.stderr
    );
    assert!(
        !run.log()["runs"][0]["invocations"][0]["executionSuccessful"]
            .as_bool()
            .expect("the invocation says whether weed ran"),
        "the log says the run never judged anything"
    );
}

#[test]
fn an_unknown_ref_exits_three() {
    let repo = Repo::init();
    let run = repo.weed(&["check", "--base", "origin/does-not-exist"]);

    assert_eq!(run.code, 3);
    assert_eq!(run.stderr_lines().len(), 1, "one line, naming the cause");
    assert!(
        run.stderr.contains("origin/does-not-exist"),
        "the line names the ref: {}",
        run.stderr
    );
}

#[test]
fn a_malformed_weed_toml_exits_three() {
    let repo = Repo::init();
    repo.write("weed.toml", "[rules]\nG1 = \"loud\"\n");

    let run = repo.weed(&["check"]);
    assert_eq!(run.code, 3);
    assert_eq!(run.stderr_lines().len(), 1, "one line, naming the cause");
    assert!(
        run.stderr.contains("G1"),
        "the line names the key that is wrong: {}",
        run.stderr
    );
}

#[test]
fn a_config_weed_cannot_read_exits_three() {
    let repo = Repo::init();
    let run = repo.weed(&["check", "--config", "nowhere/weed.toml"]);

    assert_eq!(run.code, 3);
    assert_eq!(run.stderr_lines().len(), 1, "one line, naming the cause");
    assert!(
        run.stderr.contains("nowhere/weed.toml"),
        "the line names the file it could not read: {}",
        run.stderr
    );
}
