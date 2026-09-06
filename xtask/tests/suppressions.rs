//! `cargo xtask suppressions` on a history that installs guard part way through.
//!
//! The garden repositories have not installed guard yet, so every rate they
//! carry today is zero. A measurement that can only answer zero proves nothing,
//! so the suite builds the history the rate is defined over: allowances written
//! before the hooks existed, the commit that brings them in, and allowances
//! written after.

mod common;

use common::{workspace, xtask, xtask_under, Repo};

use serde_json::Value;

/// What `weed guard install` writes into every bundle it creates. The
/// measurement finds the install by this line, and so does the suite.
const MARKER: &str = "# weed-guard-binary:";

fn rates(repo: &Repo) -> Value {
    let run = xtask(&[
        "suppressions",
        "--repo",
        &format!("probe={}", repo.root().display()),
        "--only",
        "probe",
        "--format",
        "json",
    ]);
    run.succeeded();
    serde_json::from_str::<Value>(&run.stdout)
        .expect("the measurement writes json")
        .as_array()
        .and_then(|rates| rates.first().cloned())
        .expect("one repository was asked about")
}

/// Two commits with an allowance each before the hooks arrive, the install, and
/// four commits after it of which one carries two allowances.
fn probe() -> Repo {
    let repo = Repo::init();
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.commit("the repository begins");
    repo.write("src/two.rs", "pub fn two() -> u32 {\n    2\n}\n");
    repo.commit("a change\n\nWeed-allow: T1 written when there was no gate to allow past");
    repo.write("src/three.rs", "pub fn three() -> u32 {\n    3\n}\n");
    repo.commit("another change\n\nWeed-allow: T2 also written before the hooks");

    repo.write(
        ".githooks/pre-commit",
        &format!("#!/bin/sh\n{MARKER} /usr/local/bin/weed\nexec weed guard pre-commit\n"),
    );
    repo.commit("guard is installed");

    repo.write("src/four.rs", "pub fn four() -> u32 {\n    4\n}\n");
    repo.commit("a change with no allowance");
    repo.write("src/five.rs", "pub fn five() -> u32 {\n    5\n}\n");
    repo.commit("a change\n\nWeed-allow: T1 the case moved to the module beside it");
    repo.write("src/six.rs", "pub fn six() -> u32 {\n    6\n}\n");
    repo.commit(
        "a change\n\nWeed-allow: T2 the assertion moved into the helper\nWeed-allow: S1 the marker is in a fixture",
    );
    repo
}

#[test]
fn the_rate_starts_the_day_guard_is_installed() {
    let repo = probe();
    let rate = rates(&repo);

    assert_eq!(rate["repo"], "probe");
    assert_eq!(rate["commits"], 7, "the whole branch is the ground");
    assert_eq!(
        rate["commits_since_install"], 4,
        "the install itself and the three commits after it"
    );
    assert_eq!(
        rate["trailers_since_install"], 3,
        "one allowance on one commit and two on another"
    );
    assert_eq!(
        rate["trailers_before_install"], 2,
        "the allowances written before the hooks are counted apart and not in the rate"
    );
    assert_eq!(
        rate["per_hundred_commits"], 75.0,
        "three allowances over four commits is 75 per hundred"
    );
    assert_eq!(
        rate["installed"]["sha"],
        Value::String(repo.git(&["rev-parse", "HEAD~3"]).trim().to_string()),
        "the install is the commit that brought the hook bundle in"
    );
}

#[test]
fn a_repository_that_never_installed_guard_has_no_rate_to_report() {
    let repo = Repo::init();
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.commit("the repository begins");
    repo.write("src/two.rs", "pub fn two() -> u32 {\n    2\n}\n");
    repo.commit("a change\n\nWeed-allow: T1 an allowance against a gate that is not running");

    let rate = rates(&repo);

    assert_eq!(rate["installed"], Value::Null, "guard was never installed");
    assert_eq!(rate["commits_since_install"], 0);
    assert_eq!(rate["trailers_since_install"], 0);
    assert_eq!(
        rate["trailers_before_install"], 1,
        "the allowance is still counted and reported, outside the rate"
    );
    assert_eq!(rate["per_hundred_commits"], 0.0);
}

#[test]
fn the_repository_being_read_is_left_exactly_as_it_was() {
    let repo = probe();
    let before = repo.fingerprint();

    rates(&repo);

    assert_eq!(
        before,
        repo.fingerprint(),
        "the rate reads a repository and writes nothing to it"
    );
}

#[test]
fn the_table_says_the_same_as_the_json() {
    let repo = probe();
    let run = xtask(&[
        "suppressions",
        "--repo",
        &format!("probe={}", repo.root().display()),
        "--only",
        "probe",
    ]);
    run.succeeded();

    let row = run
        .stdout
        .lines()
        .find(|line| line.starts_with("probe"))
        .expect("a row for the repository");
    let fields: Vec<&str> = row.split_whitespace().collect();
    assert_eq!(fields[1], "7", "commits on the branch");
    assert_eq!(fields[2], "4", "commits since the install");
    assert_eq!(fields[3], "3", "allowances since the install");
    assert_eq!(fields[4], "75.0", "allowances per hundred commits");
}

#[test]
fn the_scratch_a_run_fetches_into_is_removed_when_it_ends() {
    let repo = probe();
    let scratch = workspace();

    xtask_under(
        &[
            "suppressions",
            "--repo",
            &format!("probe={}", repo.root().display()),
            "--only",
            "probe",
        ],
        scratch.path(),
    )
    .succeeded();

    let left: Vec<String> = std::fs::read_dir(scratch.path())
        .expect("the scratch directory should be readable")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        left.is_empty(),
        "the run put its scratch repositories here and left {left:?} behind"
    );
}

#[test]
fn a_workspace_the_measurement_never_saw_is_refused() {
    let run = workspace();
    let missing = run.path().join("no-such-checkout");

    let refused = xtask(&[
        "suppressions",
        "--repo",
        &format!("probe={}", missing.display()),
        "--only",
        "probe",
    ]);
    refused.failed();
    assert!(
        refused.stderr.contains("there is no git repository there"),
        "a rate over a repository that is not there is not a rate: {}",
        refused.stderr
    );
}
