//! The verdict's floor, drawn from the blind re-grade's own table.
//!
//! A reader handed one share has to take the classification behind it on trust,
//! because whether a block was a false positive is a judgement and the number
//! rests on it. Handed two, they can see how far the share moves under the
//! harshest reading anybody wrote down: the ledger's own, and the same
//! measurement with every `false-positive` a blind auditor recorded believed
//! over it.
//!
//! So the floor is read off that auditor's table and never typed. These suites
//! judge a history built for the purpose, where every commit is known, and move
//! only the re-grade beside it.

mod common;

use common::{three_blocking_commits, Bench, Repo};

/// A sample wide enough and agreeing well enough to stand on its own, so the
/// floor is the only thing these suites are moving.
const AT_THE_BAR: [(&str, usize, usize); 1] = [("blocked commits", 20, 20)];

/// The paragraph after the verdict, which is where the floor is printed.
fn after_the_verdict(bench: &Bench) -> String {
    let paragraphs = bench.paragraphs();
    assert!(
        paragraphs.len() > 2,
        "the report should carry a paragraph after the verdict:\n{}",
        bench.report()
    );
    paragraphs[2].clone()
}

/// The three blocked commits, oldest first.
fn blocked(repo: &Repo) -> Vec<String> {
    repo.git(&["rev-list", "--reverse", "main"])
        .lines()
        .skip(1)
        .map(str::to_string)
        .collect()
}

/// The bench, with the three blocks classified as the caller says.
fn ledger(repo: &Repo, classes: [&str; 3]) -> (Bench, Vec<String>) {
    let bench = Bench::new();
    let shas = blocked(repo);
    let mut text = String::new();
    for (sha, class) in shas.iter().zip(classes) {
        text.push_str(&format!(
            "[[commit]]\nrepo = \"probe\"\nsha = \"{sha}\"\nclassification = \"{class}\"\nreasoning = \"a case declared in that file stopped being declared\"\n\n"
        ));
    }
    std::fs::write(bench.judgements(), text).expect("the ledger should be writable");
    (bench, shas)
}

#[test]
fn with_no_blind_re_grade_the_floor_is_the_ledgers_own_share() {
    let repo = three_blocking_commits();
    let (bench, _) = ledger(&repo, ["false-positive", "acceptable", "true-positive"]);

    bench.calibrate("probe", &repo, &[]).succeeded();

    let after = after_the_verdict(&bench);
    assert!(
        after.contains("No blind re-grade has read a blocked commit back"),
        "with no blind re-grade the paragraph says so:\n{after}"
    );
    assert!(
        after.contains("the ledger's own 33.33 percent, 1 of 3 commits judged"),
        "one of three blocks is a false positive:\n{after}"
    );
}

#[test]
fn a_blind_false_positive_lifts_the_floor_above_the_ledger() {
    let repo = three_blocking_commits();
    let (bench, shas) = ledger(&repo, ["false-positive", "acceptable", "true-positive"]);
    bench.write_audit_regrade(
        "audit-blind.md",
        Some("yes"),
        &AT_THE_BAR,
        &[
            ("probe", &shas[1][..10], "false-positive"),
            ("probe", &shas[2][..10], "acceptable"),
        ],
    );

    bench.calibrate("probe", &repo, &[]).succeeded();

    let after = after_the_verdict(&bench);
    assert!(
        after.contains("The floor under that share is 66.67 percent, 2 of the 3 commits judged"),
        "the auditor's false positive joins the ledger's:\n{after}"
    );
    assert!(
        after.contains("the ledger's own reading is 33.33 percent, 1 of the same 3"),
        "the ledger's own share is printed beside it:\n{after}"
    );
    assert!(
        after.contains("audit-blind.md"),
        "the paragraph names the re-grade it drew the floor from:\n{after}"
    );
}

#[test]
fn the_floor_moves_with_the_table_and_not_with_the_prose() {
    let repo = three_blocking_commits();
    let (bench, shas) = ledger(&repo, ["false-positive", "acceptable", "true-positive"]);

    bench.write_audit_regrade(
        "audit-blind.md",
        Some("yes"),
        &AT_THE_BAR,
        &[("probe", &shas[1][..10], "acceptable")],
    );
    bench.calibrate("probe", &repo, &[]).succeeded();
    let agreeing = after_the_verdict(&bench);

    bench.write_audit_regrade(
        "audit-blind.md",
        Some("yes"),
        &AT_THE_BAR,
        &[
            ("probe", &shas[1][..10], "false-positive"),
            ("probe", &shas[2][..10], "false-positive"),
        ],
    );
    bench.calibrate("probe", &repo, &[]).succeeded();
    let harsher = after_the_verdict(&bench);

    assert!(
        agreeing.contains("The floor under that share is 33.33 percent"),
        "an auditor who calls nothing a false positive leaves the floor at the ledger's share:\n{agreeing}"
    );
    assert!(
        harsher.contains("The floor under that share is 100.00 percent, 3 of the 3 commits judged"),
        "an auditor who refuses every block puts the floor at all of them:\n{harsher}"
    );
    assert_ne!(
        agreeing, harsher,
        "the paragraph has to change when the re-grade's table does"
    );
}

#[test]
fn a_sighted_false_positive_does_not_move_the_floor() {
    let repo = three_blocking_commits();
    let (bench, shas) = ledger(&repo, ["false-positive", "acceptable", "true-positive"]);
    bench.write_audit_regrade(
        "audit-sighted.md",
        Some("no; the auditor could read the builder's classification"),
        &AT_THE_BAR,
        &[
            ("probe", &shas[1][..10], "false-positive"),
            ("probe", &shas[2][..10], "false-positive"),
        ],
    );

    bench.calibrate("probe", &repo, &[]).succeeded();

    let after = after_the_verdict(&bench);
    assert!(
        after.contains("No blind re-grade has read a blocked commit back"),
        "a sighted auditor hands the builder's own answer back, so no floor is drawn from it:\n{after}"
    );
}

#[test]
fn a_re_grade_naming_a_commit_the_ledger_already_refused_moves_nothing() {
    let repo = three_blocking_commits();
    let (bench, shas) = ledger(&repo, ["false-positive", "acceptable", "true-positive"]);
    bench.write_audit_regrade(
        "audit-blind.md",
        Some("yes"),
        &AT_THE_BAR,
        &[("probe", &shas[0][..10], "false-positive")],
    );

    bench.calibrate("probe", &repo, &[]).succeeded();

    let after = after_the_verdict(&bench);
    assert!(
        after.contains("The floor under that share is 33.33 percent, 1 of the 3 commits judged"),
        "a block counted against the bar once is not counted twice:\n{after}"
    );
}

#[test]
fn an_unclassified_block_is_under_the_floor_as_well_as_the_share() {
    let repo = three_blocking_commits();
    let bench = Bench::new();
    let shas = blocked(&repo);
    std::fs::write(
        bench.judgements(),
        format!(
            "[[commit]]\nrepo = \"probe\"\nsha = \"{}\"\nclassification = \"acceptable\"\nreasoning = \"a case declared in that file stopped being declared\"\n",
            shas[0]
        ),
    )
    .expect("the ledger should be writable");
    bench.write_audit_regrade(
        "audit-blind.md",
        Some("yes"),
        &AT_THE_BAR,
        &[("probe", &shas[0][..10], "acceptable")],
    );

    bench.calibrate("probe", &repo, &[]).succeeded();

    let after = after_the_verdict(&bench);
    assert!(
        after.contains("The floor under that share is 66.67 percent, 2 of the 3 commits judged"),
        "the two blocks nobody classified are under the floor:\n{after}"
    );
    assert!(
        after.contains("the ledger's own reading is 66.67 percent, 2 of the same 3"),
        "and under the ledger's own share as well:\n{after}"
    );
}
