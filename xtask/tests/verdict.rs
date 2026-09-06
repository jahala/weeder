//! The first sentence is written from the re-grade, never typed.
//!
//! The classification a calibration rests on is a judgement, and the builder's
//! own judgement is not what weed ships on. So the verdict carries a
//! qualification until a second party has read the classifications back and
//! agreed, and the generator decides that from the audit file: nobody can
//! promote a provisional verdict by editing the report, because the next run
//! writes it again from the same file.

mod common;

use common::{suite, Bench, Repo};

const PROVISIONAL: &str = "weed ships as a gate, pending the independent re-grade:";
const CONFIRMED: &str = "weed ships as a gate:";

/// A repository with one blocking commit, so there is something to classify and
/// the verdict is a ship rather than a kill.
fn probe() -> Repo {
    let repo = Repo::init();
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.write("tests/unit.rs", &suite(3));
    repo.commit("the repository begins");

    repo.write("tests/unit.rs", &suite(2));
    repo.commit("one case fewer");
    repo
}

/// The bench, with its one block classified, so the numbers clear the bar and
/// only the re-grade decides the wording.
fn cleared() -> (Repo, Bench) {
    let repo = probe();
    let bench = Bench::new();
    std::fs::write(
        bench.judgements(),
        format!(
            "[[commit]]\nrepo = \"probe\"\nsha = \"{}\"\nclassification = \"true-positive\"\nreasoning = \"a case really did go\"\n",
            repo.tip()
        ),
    )
    .expect("the ledger should be writable");
    (repo, bench)
}

#[test]
fn without_a_re_grade_the_verdict_is_pending() {
    let (repo, bench) = cleared();

    bench.calibrate("probe", &repo, &[]).succeeded();

    let verdict = bench.verdict();
    assert!(
        verdict.starts_with(PROVISIONAL),
        "no second party has read the classifications back: {verdict}"
    );
    assert!(
        bench.report().contains("is not written yet"),
        "the report says why the verdict is pending:\n{}",
        bench.report()
    );
}

#[test]
fn a_re_grade_at_the_bar_takes_the_qualification_off() {
    let (repo, bench) = cleared();
    bench.write_audit(&[("blocked commits", 20, 19), ("recall cases", 24, 24)]);

    bench.calibrate("probe", &repo, &[]).succeeded();

    let verdict = bench.verdict();
    assert!(
        verdict.starts_with(CONFIRMED),
        "every sample agrees at or above ninety percent: {verdict}"
    );
    assert!(
        !verdict.starts_with(PROVISIONAL),
        "the qualification is gone, not merely reworded: {verdict}"
    );
    assert!(
        bench
            .report()
            .contains("blocked commits at 95.0 percent of 20 cases"),
        "the report says what the re-grade found:\n{}",
        bench.report()
    );
}

#[test]
fn one_sample_under_the_bar_keeps_the_verdict_pending() {
    let (repo, bench) = cleared();
    bench.write_audit(&[("blocked commits", 20, 20), ("recall cases", 20, 17)]);

    bench.calibrate("probe", &repo, &[]).succeeded();

    let verdict = bench.verdict();
    assert!(
        verdict.starts_with(PROVISIONAL),
        "the bar is on each sample, not on the two of them averaged: {verdict}"
    );
    assert!(
        bench
            .report()
            .contains("recall cases agrees on 85.0 percent of 20 cases"),
        "the report names the sample that fell short:\n{}",
        bench.report()
    );
}

#[test]
fn a_sample_too_small_to_rest_on_keeps_the_verdict_pending() {
    let (repo, bench) = cleared();
    bench.write_audit(&[("blocked commits", 3, 3)]);

    bench.calibrate("probe", &repo, &[]).succeeded();

    let verdict = bench.verdict();
    assert!(
        verdict.starts_with(PROVISIONAL),
        "three cases at a hundred percent is not a re-grade: {verdict}"
    );
    assert!(
        bench.report().contains("the sample floor is 20"),
        "the report says the sample was too small:\n{}",
        bench.report()
    );
}

#[test]
fn an_audit_drawing_no_sample_keeps_the_verdict_pending() {
    let (repo, bench) = cleared();
    std::fs::write(
        bench.audit(),
        "# the re-grade\n\nnobody has done this yet.\n",
    )
    .expect("the audit should be writable");

    bench.calibrate("probe", &repo, &[]).succeeded();

    assert!(
        bench.verdict().starts_with(PROVISIONAL),
        "a file with no agreement in it records no agreement: {}",
        bench.verdict()
    );
}

#[test]
fn the_wording_is_written_from_the_file_and_not_from_the_report() {
    let (repo, bench) = cleared();
    bench.write_audit(&[("blocked commits", 20, 20)]);
    bench.calibrate("probe", &repo, &[]).succeeded();
    assert!(bench.verdict().starts_with(CONFIRMED));

    // Somebody edits the report to say the re-grade is still pending, and
    // somebody else takes the re-grade away. The next run writes whichever
    // sentence the audit file earns, either way.
    let promoted = bench.report().replace(CONFIRMED, PROVISIONAL);
    std::fs::write(bench.out(), &promoted).expect("the report should be writable");
    std::fs::remove_file(bench.audit()).expect("the audit should be removable");

    bench.calibrate("probe", &repo, &[]).succeeded();
    assert!(
        bench.verdict().starts_with(PROVISIONAL),
        "with the re-grade gone the qualification comes back: {}",
        bench.verdict()
    );

    bench.write_audit(&[("blocked commits", 20, 20)]);
    bench.calibrate("probe", &repo, &[]).succeeded();
    assert!(
        bench.verdict().starts_with(CONFIRMED),
        "with the re-grade back the qualification goes again, and nobody edited the report: {}",
        bench.verdict()
    );
}

#[test]
fn a_run_that_does_not_clear_the_bar_says_so_whatever_the_re_grade_says() {
    let repo = probe();
    let bench = Bench::new();
    bench.write_audit(&[("blocked commits", 20, 20)]);

    bench.calibrate("probe", &repo, &[]).succeeded();

    assert!(
        bench.verdict().starts_with("weed does not ship as a gate:"),
        "an unread block is a false positive, and no re-grade turns that into a ship: {}",
        bench.verdict()
    );
}
