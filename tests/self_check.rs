//! weeder, judged by weeder.
//!
//! `--base <root commit>` is the widest question this repository can be asked:
//! the root commit carries one empty file, so every line weeder has ever written
//! reads as added and every check rule runs over the whole tree at once. A gate
//! its own author cannot pass is a gate nobody else will adopt, so nothing here
//! may come back at block level.
//!
//! Two things earn that clean run, and both are written down rather than turned
//! off. `[scope] specimens` excludes `fixtures/adversarial`, whose files are
//! written to look dishonest on purpose; every path it covers is still reported,
//! once, as a note. And `weeder.toml` carries an allowance for C1 with its reason,
//! because a guardrail arriving with the repository that has never had one is
//! not a gate being loosened. The third test here holds that allowance to its
//! word: an ordinary edit to the same file still blocks.
//!
//! The log is validated against the schema this repository vendored, read from
//! the bytes the binary wrote rather than from anything built in process.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use jsonschema::Validator;

/// weeder's own law, and the file the third test edits.
const CONFIG: &str = "weeder.toml";

/// weeder's own repository, the tree under judgement.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The commit this repository begins at. `git rev-list` answers newest first,
/// so the oldest root is the last line, and a repository with one root has only
/// that line to give.
fn root_commit() -> String {
    let run = common::git_in(&repository(), &["rev-list", "--max-parents=0", "HEAD"]);
    assert_eq!(
        run.code, 0,
        "the repository should name its root commit: {}",
        run.stderr
    );
    run.stdout
        .split_whitespace()
        .last()
        .expect("a repository has at least one root commit")
        .to_string()
}

/// The run the check is about: weeder judging its own tree against its first
/// commit, writing SARIF.
fn self_check() -> common::Run {
    let base = root_commit();
    common::weeder_in(
        &repository(),
        &["check", "--base", &base, "--format", "sarif"],
    )
}

fn validator() -> Validator {
    let path = repository().join("schemas/sarif-schema-2.1.0.json");
    let source = fs::read_to_string(&path).expect("the vendored schema should be readable");
    let schema = serde_json::from_str(&source).expect("the vendored schema should be json");
    jsonschema::validator_for(&schema).expect("the vendored schema should compile")
}

#[test]
fn weeder_judging_its_own_tree_from_the_root_commit_reports_nothing_at_block_level() {
    let run = self_check();

    let blocked: Vec<String> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.level == "error")
        .map(|finding| {
            format!(
                "{} {}:{} {}",
                finding.rule,
                finding.path,
                finding.line.unwrap_or_default(),
                finding.message
            )
        })
        .collect();
    assert!(
        blocked.is_empty(),
        "weeder blocks on its own tree:\n{}",
        blocked.join("\n")
    );
    assert_eq!(
        run.code, 0,
        "nothing at block level leaves with 0: {}",
        run.stderr
    );

    // A run that judged nothing would pass this check by saying nothing at all.
    assert!(
        !run.findings().is_empty(),
        "the whole repository read as one change has warnings and notes to report"
    );
}

#[test]
fn the_log_weeder_writes_about_itself_validates_against_the_vendored_schema() {
    let run = self_check();
    let value: serde_json::Value = serde_json::from_str(&run.stdout).unwrap_or_else(|error| {
        panic!(
            "weeder should write parseable json: {error}\n{}",
            run.stdout
        )
    });

    let validator = validator();
    let complaints: Vec<String> = validator
        .iter_errors(&value)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(
        complaints.is_empty(),
        "the log weeder wrote about itself does not validate:\n{}",
        complaints.join("\n")
    );
}

#[test]
fn the_allowance_the_config_carries_does_not_disarm_c1_for_an_ordinary_edit() {
    let config =
        fs::read_to_string(repository().join(CONFIG)).expect("weeder.toml should be readable");
    assert!(
        config.contains("weeder-allow C1"),
        "the clean run rests on this allowance, so the test that bounds it needs it there"
    );

    let repo = common::Repo::init();
    repo.write(CONFIG, &config);
    repo.write("src/main.rs", "fn main() {}\n");
    repo.commit("the repository already carries its config");

    let edited = config.replace(
        "cli = [\"src/main.rs\"]",
        "cli = [\"src/main.rs\", \"src/faces/mod.rs\"]",
    );
    assert_ne!(edited, config, "the edit has to change the file");
    repo.write(CONFIG, &edited);
    repo.stage_all();

    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(
        run.code, 2,
        "an ordinary edit to weeder's own config is still a guardrail edit: {}",
        run.stderr
    );
    let findings = run.findings();
    assert!(
        findings.iter().any(|finding| finding.rule == "C1"
            && finding.path == CONFIG
            && finding.level == "error"),
        "C1 blocks on the edit, allowance or no allowance: {findings:#?}"
    );
}
