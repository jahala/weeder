//! The anchoring caveat, written from the re-grades' own declarations.
//!
//! An agreement is worth what the auditor could not see. A second party who
//! read the builder's class beside each case before judging agreed with
//! something already in front of them, and forty of forty taken that way reads
//! stronger than it is. So the paragraph the verdict opens says which kind of
//! re-grade the number rests on, and it says it from the `Blind:` line each
//! audit file writes about itself: a re-grade redone blind changes the sentence
//! by being written, never by being described.

mod common;

use common::{cleared, Bench};

/// The paragraph the report opens with: the verdict, and then the caveat.
fn opening(bench: &Bench) -> String {
    let paragraphs = bench.paragraphs();
    assert!(
        paragraphs.len() > 1,
        "the report should carry a verdict paragraph:\n{}",
        bench.report()
    );
    paragraphs[1].clone()
}

/// A sample every audit in these suites draws, wide enough and agreeing well
/// enough to stand on its own.
const AT_THE_BAR: [(&str, usize, usize); 1] = [("blocked commits", 20, 20)];

#[test]
fn a_sighted_re_grade_is_named_as_sighted() {
    let (repo, bench) = cleared();
    bench.write_audit_record(
        "audit-one.md",
        Some("no; the auditor could read the builder's classification"),
        &AT_THE_BAR,
    );

    bench.calibrate("probe", &repo, &[]).succeeded();

    let opening = opening(&bench);
    assert!(
        opening.contains("The re-grade behind it is a sighted one"),
        "the only re-grade declares itself not blind:\n{opening}"
    );
    assert!(
        opening.contains("could read the builder's class beside each case before judging"),
        "the caveat says what the auditor could see:\n{opening}"
    );
    assert!(
        opening.contains("audit-one.md"),
        "the caveat names the file it read the declaration from:\n{opening}"
    );
}

#[test]
fn the_caveat_sits_in_the_first_paragraph_right_after_the_verdict() {
    let (repo, bench) = cleared();
    bench.write_audit_record("audit-one.md", Some("no"), &AT_THE_BAR);

    bench.calibrate("probe", &repo, &[]).succeeded();

    let opening = opening(&bench);
    let verdict = bench.verdict();
    assert!(
        verdict.starts_with("weeder ships as a gate"),
        "the verdict opens the paragraph: {verdict}"
    );
    let caveat = "The re-grade behind it is a sighted one";
    let at = opening
        .find(caveat)
        .unwrap_or_else(|| panic!("the caveat should be in the opening paragraph:\n{opening}"));
    let before = opening[..at].trim_end();
    // Under the ruling of 2026-09-06 a sighted re-grade alone cannot lift the
    // qualification, so the verdict sentence may end on the pending tail; the
    // caveat still follows it with nothing in between.
    assert!(
        before.starts_with("weeder ships as a gate")
            && (before.ends_with("all still at block level.")
                || before.ends_with("until a re-grade agrees at the bar.")),
        "nothing stands between the verdict and the caveat: {before}"
    );
    assert!(
        !bench.report()[..bench.report().find(caveat).unwrap_or_default()].contains("\n\n## "),
        "the caveat comes before any section of the report:\n{}",
        bench.report()
    );
}

#[test]
fn a_blind_re_grade_at_the_bar_is_named_beside_the_sighted_one() {
    let (repo, bench) = cleared();
    bench.write_audit_record("audit-a.md", Some("no"), &AT_THE_BAR);
    bench.write_audit_record("audit-b.md", Some("yes"), &AT_THE_BAR);

    bench.calibrate("probe", &repo, &[]).succeeded();

    let opening = opening(&bench);
    assert!(
        opening.contains("The re-grade behind it is blind"),
        "a blind re-grade at the bar is what the number rests on now:\n{opening}"
    );
    assert!(
        opening.contains("audit-b.md") && opening.contains("audit-a.md"),
        "the caveat names both re-grades, the blind one and the sighted one:\n{opening}"
    );
    assert!(
        !opening.contains("The re-grade behind it is a sighted one"),
        "the agreement is no longer only a sighted one:\n{opening}"
    );
}

#[test]
fn a_blind_re_grade_under_the_bar_leaves_the_agreement_sighted() {
    let (repo, bench) = cleared();
    bench.write_audit_record("audit-a.md", Some("no"), &AT_THE_BAR);
    bench.write_audit_record("audit-b.md", Some("yes"), &[("blocked commits", 20, 9)]);

    bench.calibrate("probe", &repo, &[]).succeeded();

    let opening = opening(&bench);
    assert!(
        opening.contains("The re-grade behind it is a sighted one"),
        "a blind re-grade that does not agree carries nothing:\n{opening}"
    );
    assert!(
        opening.contains("audit-b.md is under the bar"),
        "the caveat says the blind re-grade is there and short of the bar:\n{opening}"
    );
    assert!(
        !opening.contains("The re-grade behind it is blind"),
        "an under-bar blind re-grade does not make the number a blind one:\n{opening}"
    );
}

#[test]
fn the_caveat_is_read_from_the_declaration_and_not_from_the_file_name() {
    let (repo, bench) = cleared();
    // A file whose name says blind and whose declaration says otherwise. The
    // report has to believe the declaration.
    let path = bench.write_audit_record("audit-blind.md", Some("no"), &AT_THE_BAR);

    bench.calibrate("probe", &repo, &[]).succeeded();
    let sighted = opening(&bench);
    assert!(
        sighted.contains("The re-grade behind it is a sighted one"),
        "the name says blind and the file says it is not:\n{sighted}"
    );

    let flipped = std::fs::read_to_string(&path)
        .expect("the audit should be readable")
        .replace("Blind: no", "Blind: yes");
    std::fs::write(&path, flipped).expect("the audit should be writable");

    bench.calibrate("probe", &repo, &[]).succeeded();
    let blind = opening(&bench);
    assert!(
        blind.contains("The re-grade behind it is blind"),
        "the same file, redeclared blind, changes the sentence:\n{blind}"
    );
    assert_ne!(
        sighted, blind,
        "the caveat is written from the declaration, so changing it changes the paragraph"
    );
}

#[test]
fn a_re_grade_that_declares_nothing_is_reported_as_declaring_nothing() {
    let (repo, bench) = cleared();
    bench.write_audit_record("audit-one.md", None, &AT_THE_BAR);

    bench.calibrate("probe", &repo, &[]).succeeded();

    let opening = opening(&bench);
    assert!(
        opening.contains("audit-one.md declares no `Blind:` line either way"),
        "a file that says nothing about sight is not read as either:\n{opening}"
    );
    assert!(
        !opening.contains("The re-grade behind it is a sighted one")
            && !opening.contains("The re-grade behind it is blind"),
        "an undeclared re-grade is not guessed at:\n{opening}"
    );
}

#[test]
fn with_no_re_grade_the_paragraph_says_nothing_stands_behind_the_number() {
    let (repo, bench) = cleared();

    bench.calibrate("probe", &repo, &[]).succeeded();

    let opening = opening(&bench);
    assert!(
        opening.contains("No re-grade is recorded"),
        "nobody has re-graded anything, sighted or blind:\n{opening}"
    );
}
