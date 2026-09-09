//! `cargo xtask suppressions` on a history that installs guard part way through.
//!
//! The garden repositories have not installed guard yet, so every rate they
//! carry today is zero. A measurement that can only answer zero proves nothing,
//! so the suite builds the history the rate is defined over: allowances written
//! before the hooks existed, the commit that brings them in, and allowances
//! written after.
//!
//! The history is written in both spellings of the trailer, the tool's own and
//! the one it carried before the rename to weeder. A history read is not the
//! gate: an allowance written under the old name still let a finding past a gate
//! that was running, so both count, and they count into one rate rather than two
//! columns. What the gate itself honours is a different question, and
//! `tests/suppress_token.rs` is where it is asked.

mod common;

use common::{workspace, xtask, Bench, Repo};

use serde_json::Value;

/// What `weeder guard install` writes into every bundle it creates. The
/// measurement finds the install by this line, and so does the suite.
const MARKER: &str = "# weeder-guard-binary:";

fn rates(repo: &Repo) -> Value {
    let bench = Bench::new();
    let run = bench.suppressions("probe", repo, &repo.tip(), &["--format", "json"]);
    run.succeeded();
    serde_json::from_str::<Value>(&run.stdout)
        .expect("the measurement writes json")
        .as_array()
        .and_then(|rates| rates.first().cloned())
        .expect("one repository was asked about")
}

/// The trailer the tool carries, and the one it carried before the rename. The
/// retired spelling is built rather than written, so this file is not itself a
/// place it lives on.
fn tokens() -> (String, String) {
    let stem = "weed";
    (
        format!("W{}er-allow:", &stem[1..]),
        format!("W{}-allow:", &stem[1..]),
    )
}

/// Two commits with an allowance each before the hooks arrive, the install, and
/// four commits after it of which one carries two allowances. Both spellings run
/// through the history on either side of the install, because both are what a
/// repository that lived through the rename actually carries.
fn probe() -> Repo {
    let (own, retired) = tokens();
    let repo = Repo::init();
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.commit("the repository begins");
    repo.write("src/two.rs", "pub fn two() -> u32 {\n    2\n}\n");
    repo.commit(&format!(
        "a change\n\n{own} T1 written when there was no gate to allow past"
    ));
    repo.write("src/three.rs", "pub fn three() -> u32 {\n    3\n}\n");
    repo.commit(&format!(
        "another change\n\n{retired} T2 also written before the hooks"
    ));

    repo.write(
        ".githooks/pre-commit",
        &format!("#!/bin/sh\n{MARKER} /usr/local/bin/weeder\nexec weeder guard pre-commit\n"),
    );
    repo.commit("guard is installed");

    repo.write("src/four.rs", "pub fn four() -> u32 {\n    4\n}\n");
    repo.commit("a change with no allowance");
    repo.write("src/five.rs", "pub fn five() -> u32 {\n    5\n}\n");
    repo.commit(&format!(
        "a change\n\n{own} T1 the case moved to the module beside it"
    ));
    repo.write("src/six.rs", "pub fn six() -> u32 {\n    6\n}\n");
    repo.commit(&format!(
        "a change\n\n{retired} T2 the assertion moved into the helper\n{own} S1 the marker is in a fixture"
    ));
    repo
}

/// The same history, written in one spelling throughout. Two of these, one per
/// spelling, are the whole claim: the rate does not know which name the tool
/// carried on the day an allowance was written.
fn one_spelling(trailer: &str) -> Repo {
    let repo = Repo::init();
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.commit("the repository begins");
    repo.write(
        ".githooks/pre-commit",
        &format!("#!/bin/sh\n{MARKER} /usr/local/bin/weeder\nexec weeder guard pre-commit\n"),
    );
    repo.commit("guard is installed");
    repo.write("src/two.rs", "pub fn two() -> u32 {\n    2\n}\n");
    repo.commit(&format!(
        "a change\n\n{trailer} T1 the case moved to the module beside it"
    ));
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
    repo.commit(&format!(
        "a change\n\n{} T1 an allowance against a gate that is not running",
        tokens().1
    ));

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
    let bench = Bench::new();
    let run = bench.suppressions("probe", &repo, &repo.tip(), &[]);
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
    let bench = Bench::new();
    let scratch = workspace();

    bench.write_corpus("probe", repo.root(), &repo.tip());
    common::xtask_under(
        &[
            "suppressions",
            "--corpus",
            &bench.corpus().display().to_string(),
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
fn a_source_the_measurement_never_saw_is_refused() {
    let bench = Bench::new();
    let missing = bench.path().join("no-such-checkout");
    bench.write_corpus(
        "probe",
        &missing,
        "0123456789012345678901234567890123456789",
    );

    let refused = xtask(&[
        "suppressions",
        "--corpus",
        &bench.corpus().display().to_string(),
    ]);
    refused.failed();
    assert!(
        refused.stderr.contains("`git fetch"),
        "a rate over a repository that is not there is not a rate: {}",
        refused.stderr
    );
}

#[test]
fn a_pin_that_is_not_a_full_sha_is_refused() {
    let repo = probe();
    let bench = Bench::new();

    let refused = bench.suppressions("probe", &repo, &repo.tip()[..8], &[]);
    refused.failed();
    assert!(
        refused.stderr.contains("not a full forty-character sha"),
        "an abbreviation can come to mean a second commit: {}",
        refused.stderr
    );
}

#[test]
fn both_spellings_of_the_trailer_land_in_one_rate() {
    let (own, retired) = tokens();
    let under_the_new_name = rates(&one_spelling(&own));
    let under_the_old_name = rates(&one_spelling(&retired));

    assert_eq!(
        under_the_old_name["trailers_since_install"], 1,
        "an allowance written under the name the tool used to carry still let a finding past"
    );
    // Everything the rate is made of, side by side. The two histories differ
    // only in the word the trailer opens with, so their shas differ and nothing
    // the measurement reports may.
    for field in [
        "commits",
        "commits_since_install",
        "trailers_since_install",
        "trailers_before_install",
        "per_hundred_commits",
    ] {
        assert_eq!(
            under_the_new_name[field], under_the_old_name[field],
            "the rate is of allowances, not of spellings, and `{field}` reports one number for both"
        );
    }
}

#[test]
fn a_history_carrying_both_spellings_counts_every_allowance_once() {
    let rate = rates(&probe());

    assert_eq!(
        rate["trailers_since_install"], 3,
        "two spellings across two commits, counted as three allowances"
    );
    assert_eq!(
        rate["trailers_before_install"], 2,
        "and the two written before the hooks, one in each spelling"
    );
}
