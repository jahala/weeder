//! R3, a TODO is older than the configured age.
//!
//! The age comes from git, so the repository here is a real one with two dated
//! commits: one that left a marker long enough ago to be past the default of
//! thirty days, and one that left an identical marker today. Nothing about the
//! two files differs except when each was written, which is the whole claim.

mod common;

use std::time::{SystemTime, UNIX_EPOCH};

use common::{Finding, Repo};

/// Seconds in a day.
const DAY: u64 = 86_400;

/// How far back the old marker is written. Well past the default of thirty
/// days, and well short of anything a clock skew could close.
const LONG_AGO: u64 = 200 * DAY;

/// A file carrying a work marker in a comment, and one line of code under it so
/// the file is a program rather than a note.
fn marked(marker: &str) -> String {
    format!("// {marker}: split the record on the separator the header names\npub fn parse(line: &str) -> Vec<String> {{\n    line.split(',').map(str::to_string).collect()\n}}\n")
}

/// The date git reads for a commit written that many seconds ago.
fn seconds_ago(seconds: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the machine's clock should be after 1970")
        .as_secs();
    format!("@{} +0000", now - seconds)
}

/// A repository whose old marker was written long ago and whose fresh one was
/// written today, each in its own commit so blame dates them apart.
fn aged_repository() -> Repo {
    let repo = Repo::init();
    repo.write("src/old.rs", &marked("TODO"));
    repo.commit_dated("the old marker is written", &seconds_ago(LONG_AGO));
    repo.write("src/fresh.rs", &marked("TODO"));
    repo.commit_dated("the fresh marker is written", &seconds_ago(0));
    repo
}

fn findings(repo: &Repo, arguments: &[&str]) -> Vec<Finding> {
    let mut all = vec!["scan", "--rules", "R3", "--format", "sarif"];
    all.extend_from_slice(arguments);
    let run = repo.weeder(&all);
    assert_eq!(
        run.code, 0,
        "a scan never blocks, and this one left with {}: {}",
        run.code, run.stderr
    );
    run.findings()
}

#[test]
fn a_marker_older_than_the_threshold_is_reported_and_a_fresh_one_is_not() {
    let repo = aged_repository();
    let found = findings(&repo, &[]);
    assert_eq!(
        found.len(),
        1,
        "only the old marker is past the threshold, and weeder reported: {found:#?}"
    );
    let finding = &found[0];
    assert_eq!(finding.rule, "R3");
    assert_eq!(finding.level, "warning", "a scan finding never blocks");
    assert_eq!(finding.path, "src/old.rs");
    assert_eq!(finding.line, Some(1), "the marker sits on the first line");
    assert!(
        finding.message.contains("TODO"),
        "the finding should name the marker: {}",
        finding.message
    );
}

#[test]
fn the_threshold_is_the_repository_s_own() {
    let repo = aged_repository();
    repo.write("weeder.toml", "[thresholds]\ntodo_age_days = 3650\n");
    repo.commit_dated("the repository allows a decade", &seconds_ago(0));
    let found = findings(&repo, &[]);
    assert!(
        found.is_empty(),
        "a repository that allows ten years of a marker has none that are old, and weeder reported: {found:#?}"
    );
}

#[test]
fn a_marker_in_a_string_is_a_message_rather_than_a_note() {
    let repo = Repo::init();
    repo.write(
        "src/report.rs",
        "pub fn banner() -> String {\n    \"TODO is how this program spells unfinished work\".to_string()\n}\n",
    );
    repo.commit_dated("the banner is written", &seconds_ago(LONG_AGO));
    let found = findings(&repo, &[]);
    assert!(
        found.is_empty(),
        "a marker inside a literal is something the program says, and weeder reported: {found:#?}"
    );
}
