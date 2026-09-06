//! What the rules moved since the run whose blocks were written down.
//!
//! Splitting a rule in two is meant to take friction off the gate without taking
//! the finding away. The claim is cheap; the measurement is holding the same
//! files up twice and reading what weed says each time. These suites build a
//! history where the answer is known, hand the calibration a record of what an
//! earlier run refused, and read the paragraph and the table it writes.

mod common;

use common::{suite, Bench, Repo};

/// A repository with three commits worth comparing: one that changes a workflow,
/// one that takes a test case away, and one that changes prose.
fn probe() -> Repo {
    let repo = Repo::init();
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.write("tests/unit.rs", &suite(3));
    repo.write("README.md", "# probe\n\nA repository built for a suite.\n");
    repo.write(
        ".github/workflows/ci.yml",
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo test\n",
    );
    repo.commit("the repository begins");

    repo.write(
        ".github/workflows/ci.yml",
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo test --workspace\n",
    );
    repo.commit("the workflow runs the workspace");

    repo.write("tests/unit.rs", &suite(2));
    repo.commit("one case fewer");

    repo.write(
        "README.md",
        "# probe\n\nA repository built for a suite, said again.\n",
    );
    repo.commit("a line of prose");
    repo
}

fn shas(repo: &Repo) -> Vec<String> {
    repo.git(&["rev-list", "--reverse", "main"])
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

#[test]
fn a_finding_that_dropped_to_warn_is_counted_and_listed() {
    let repo = probe();
    let bench = Bench::new();
    let shas = shas(&repo);
    // The earlier run refused the workflow edit under the rule that watched
    // guardrails; today a workflow has a rule of its own and it warns.
    bench.write_first_run(&[("probe", &shas[1], "C1", ".github/workflows/ci.yml")]);

    bench.calibrate("probe", &repo, &[]).succeeded();
    let report = bench.report();

    assert!(
        report.contains("The first run refused 1 findings at block level on these commits"),
        "the paragraph beside the share says what it is comparing with:\n{report}"
    );
    assert!(
        report.contains("This run reports 0 of them at block level still, 1 at warn level"),
        "the finding moved from block to warn:\n{report}"
    );
    assert!(
        report.contains(
            "Rule by rule: 1 moved from C1 at block level to C3 at warn level, all of them on workflow files."
        ),
        "the pair of rules and the kind of file are read off the run, not assumed:\n{report}"
    );
    assert!(
        report.contains("## What the rules moved since the first run"),
        "the counts have a table under them:\n{report}"
    );
    assert!(
        report.contains("| probe | `") && report.contains("| workflow | C1 | C3 | warn |"),
        "the table carries the row the paragraph counts:\n{report}"
    );
}

#[test]
fn a_finding_the_same_rule_still_blocks_is_not_in_the_table() {
    let repo = probe();
    let bench = Bench::new();
    let shas = shas(&repo);
    bench.write_first_run(&[("probe", &shas[2], "T1", "tests/unit.rs")]);

    bench.calibrate("probe", &repo, &[]).succeeded();
    let report = bench.report();

    assert!(
        report.contains("This run reports 1 of them at block level still, 0 at warn level"),
        "a deleted case is still refused, and by the same rule:\n{report}"
    );
    assert!(
        report.contains(
            "1 of the first run's block-level findings are refused by the same rule at the same level and are left out of this table; the 0 whose answer changed are all in it."
        ),
        "nothing moved, so the table is empty and says so:\n{report}"
    );
    assert!(
        report.contains("Nothing dropped from block level to warn level."),
        "the paragraph says plainly that nothing moved:\n{report}"
    );
}

#[test]
fn a_finding_nothing_reports_any_more_is_counted_as_silent() {
    let repo = probe();
    let bench = Bench::new();
    let shas = shas(&repo);
    bench.write_first_run(&[("probe", &shas[3], "X1", "README.md")]);

    bench.calibrate("probe", &repo, &[]).succeeded();
    let report = bench.report();

    assert!(
        report.contains("0 at note level and 1 not at all"),
        "a claim nothing repeats is counted where a reader can see it:\n{report}"
    );
    assert!(
        report.contains("| X1 | nothing | nothing |"),
        "the row says the file is refused by nobody now:\n{report}"
    );
}

#[test]
fn a_finding_outside_the_window_is_named_and_not_quietly_dropped() {
    let repo = probe();
    let bench = Bench::new();
    bench.write_first_run(&[(
        "probe",
        "0123456789012345678901234567890123456789",
        "T1",
        "tests/gone.rs",
    )]);

    bench.calibrate("probe", &repo, &[]).succeeded();
    let report = bench.report();

    assert!(
        report.contains(
            "1 of the first run's block-level findings sit on commits outside this window and are not compared."
        ),
        "a comparison that quietly shrank would read as a comparison of everything:\n{report}"
    );
    assert!(
        report.contains("`tests/gone.rs`"),
        "the finding nobody could compare is named:\n{report}"
    );
}

#[test]
fn the_paragraph_is_the_table_added_up() {
    let repo = probe();
    let bench = Bench::new();
    let shas = shas(&repo);
    bench.write_first_run(&[
        ("probe", &shas[1], "C1", ".github/workflows/ci.yml"),
        ("probe", &shas[2], "T1", "tests/unit.rs"),
        ("probe", &shas[3], "X1", "README.md"),
    ]);

    bench.calibrate("probe", &repo, &[]).succeeded();
    let report = bench.report();

    let rows: Vec<&str> = report
        .lines()
        .skip_while(|line| !line.starts_with("## What the rules moved since the first run"))
        .skip(1)
        .take_while(|line| !line.starts_with("## "))
        .filter(|line| line.starts_with("| probe |"))
        .collect();
    assert_eq!(
        rows.len(),
        2,
        "two of the three findings changed their answer:\n{report}"
    );
    assert!(
        report.contains("The first run refused 3 findings at block level on these commits"),
        "the paragraph counts every recorded finding:\n{report}"
    );
    assert!(
        report.contains(
            "1 of the first run's block-level findings are refused by the same rule at the same level and are left out of this table; the 2 whose answer changed are all in it."
        ),
        "the row count is stated where a reader can check it against the table:\n{report}"
    );
}

#[test]
fn a_corpus_with_no_record_behind_it_says_so() {
    let repo = probe();
    let bench = Bench::new();

    bench.calibrate("probe", &repo, &[]).succeeded();
    let report = bench.report();

    assert!(
        report.contains("No earlier run's blocks are recorded against this corpus"),
        "an empty comparison is said out loud rather than left as a missing section:\n{report}"
    );
    assert!(
        !report.contains("## What the rules moved since the first run"),
        "there is no table where there is nothing to compare:\n{report}"
    );
}
