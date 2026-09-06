//! The corpus is pinned, and the pin is what the measurements read.
//!
//! A report taken against "whatever the default branch says today" is a
//! different report tomorrow, and a number nobody can reproduce is not a
//! measurement. So the corpus names a full sha, and these suites build a history
//! for the purpose, judge it, commit onto the source, and judge it again: the
//! report has to come out byte for byte the same.

mod common;

use common::{suite, workspace, xtask, Bench, Repo};

/// A repository whose newest commit takes a test case away, so something in the
/// window blocks and the report has a row to lose if the window moves.
fn probe() -> Repo {
    let repo = Repo::init();
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.write("tests/unit.rs", &suite(3));
    repo.commit("the repository begins");

    repo.write(
        "src/lib.rs",
        "pub fn one() -> u32 {\n    1\n}\n\npub fn two() -> u32 {\n    2\n}\n",
    );
    repo.commit("a change that takes nothing away");

    repo.write("tests/unit.rs", &suite(2));
    repo.commit("one case fewer");
    repo
}

/// A commit on the source that the pin does not reach.
fn commit_past_the_pin(repo: &Repo) {
    repo.write("tests/unit.rs", &suite(1));
    repo.commit("another case fewer, after the pin was written");
}

#[test]
fn a_commit_added_after_the_pin_does_not_change_the_report() {
    let repo = probe();
    let bench = Bench::new();
    let pin = repo.tip();

    bench.calibrate_at("probe", &repo, &pin, &[]).succeeded();
    let before = bench.report();

    commit_past_the_pin(&repo);
    bench.calibrate_at("probe", &repo, &pin, &[]).succeeded();
    let after = bench.report();

    assert_eq!(
        before, after,
        "the corpus pins a sha, so a commit made on the source afterwards is outside the window"
    );
    assert!(
        !after.contains("after the pin was written"),
        "the commit made past the pin is judged by nobody:\n{after}"
    );
}

#[test]
fn moving_the_pin_forward_is_what_takes_the_new_commit_in() {
    let repo = probe();
    let bench = Bench::new();

    bench.calibrate("probe", &repo, &[]).succeeded();
    let before = bench.report();

    commit_past_the_pin(&repo);
    bench.calibrate("probe", &repo, &[]).succeeded();
    let after = bench.report();

    assert_ne!(
        before, after,
        "a pin moved to the new tip is a new measurement, or the pin does nothing at all"
    );
    assert!(
        after.contains("after the pin was written"),
        "the commit the moved pin reaches is judged:\n{after}"
    );
}

#[test]
fn the_window_ends_at_the_pin_and_not_at_the_branch() {
    let repo = probe();
    let older = repo.git(&["rev-parse", "main~1"]).trim().to_string();
    let bench = Bench::new();

    bench.calibrate_at("probe", &repo, &older, &[]).succeeded();
    let report = bench.report();

    assert!(
        report.contains(&format!("The window ends at `{older}`")),
        "the report names the commit its window ends at:\n{report}"
    );
    assert!(
        !report.contains("one case fewer"),
        "the branch tip is past the pin, so it is not in the window:\n{report}"
    );
    assert!(
        report.contains("## probe, 1 commits judged"),
        "two commits reach the pin and one of them has a parent:\n{report}"
    );
}

#[test]
fn the_suppression_rate_is_taken_at_the_pin_too() {
    let repo = probe();
    let bench = Bench::new();
    let pin = repo.tip();

    let before = bench.suppressions("probe", &repo, &pin, &["--format", "json"]);
    before.succeeded();
    repo.write("src/late.rs", "pub fn late() -> u32 {\n    9\n}\n");
    repo.commit("a change past the pin\n\nWeed-allow: T1 written after the pin");
    let after = bench.suppressions("probe", &repo, &pin, &["--format", "json"]);
    after.succeeded();

    assert_eq!(
        before.stdout, after.stdout,
        "the rate counts the commits reaching the pin, so a commit made afterwards is not in it"
    );
    assert!(
        before.stdout.contains(&pin),
        "the measurement says which commit it was taken at:\n{}",
        before.stdout
    );
}

#[test]
fn only_the_named_repository_of_a_corpus_is_fetched() {
    let repo = probe();
    let bench = Bench::new();
    let missing = bench.path().join("no-such-checkout");
    std::fs::write(
        bench.corpus(),
        format!(
            "[[repo]]\nname = \"probe\"\nsource = \"{}\"\ntip = \"{}\"\n\n\
             [[repo]]\nname = \"absent\"\nsource = \"{}\"\ntip = \"0123456789012345678901234567890123456789\"\n",
            repo.root().display(),
            repo.tip(),
            missing.display(),
        ),
    )
    .expect("the corpus should be writable");

    xtask(&[
        "calibrate",
        "--corpus",
        &bench.corpus().display().to_string(),
        "--only",
        "probe",
        "--out",
        &bench.out().display().to_string(),
        "--judgements",
        &bench.judgements().display().to_string(),
        "--first-run",
        &bench.first_run().display().to_string(),
        "--audit",
        &bench.audit().display().to_string(),
    ])
    .succeeded();

    let report = bench.report();
    assert!(
        report.contains("## probe,"),
        "the repository asked for is judged:\n{report}"
    );
    assert!(
        !report.contains("## absent,"),
        "an entry nobody selected is never reached for:\n{report}"
    );
}

#[test]
fn the_scratch_the_run_fetches_into_is_taken_away_again() {
    let repo = probe();
    let bench = Bench::new();
    let scratch = workspace();

    bench
        .calibrate_under("probe", &repo, scratch.path())
        .succeeded();

    let left: Vec<String> = std::fs::read_dir(scratch.path())
        .expect("the scratch directory should be readable")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        left.is_empty(),
        "the run fetches into a scratch under the temp directory and left {left:?} behind"
    );
}

#[test]
fn the_shipped_corpus_names_no_path_on_this_machine() {
    let corpus = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the workspace root")
            .join("docs/calibration/corpus.toml"),
    )
    .expect("the corpus should be readable");

    let mut sources = 0;
    let mut tips = 0;
    for line in corpus.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some(value) = value_of(line, "source") {
            sources += 1;
            assert!(
                !value.starts_with('/') && !value.starts_with('~') && !value.starts_with('.'),
                "the corpus names {value}, which is a path on one machine; a source is fetchable from any of them"
            );
            assert!(
                value.contains("://") || value.contains('@'),
                "{value} does not read as something git can fetch from"
            );
        }
        if let Some(value) = value_of(line, "tip") {
            tips += 1;
            assert!(
                value.len() == 40 && value.chars().all(|character| character.is_ascii_hexdigit()),
                "{value} is not a full forty-character sha, and an abbreviation can come to mean a second commit"
            );
        }
    }
    assert!(sources > 0, "the corpus names no repository at all");
    assert_eq!(sources, tips, "every source carries a pin of its own");
}

fn value_of(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start().strip_prefix('=')?;
    Some(rest.trim().trim_matches('"').to_string())
}
