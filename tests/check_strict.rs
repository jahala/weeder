//! `--strict`, suppressions visible, and not honoured.
//!
//! A suppression travels with the change: a `Weed-allow:` trailer on the commit
//! being prepared, or an inline `weed-allow` comment beside the line. Both turn
//! a block-level finding into a note that stops nobody, and `--strict` hands the
//! finding back its own level so a reviewer sees what the pile is really made of.

mod common;

use common::{conflicted_parser, fixture, ours, separator, theirs, Repo, BRANCH};

const REASON: &str = "the merge finishes in the follow-up commit";

#[test]
fn a_commit_trailer_makes_a_block_finding_a_note_and_strict_gives_it_back() {
    let repo = fixture("G1", "ts", "fire");
    repo.pending_message(&format!("Split on semicolons\n\nWeed-allow: G1 {REASON}\n"));

    let honoured = repo.weeder(&["check"]);
    let findings = honoured.findings();
    assert!(!findings.is_empty(), "the rule still reports");
    assert!(
        findings.iter().all(|finding| finding.level == "note"),
        "a suppressed finding stays visible and stops nobody"
    );
    assert!(
        findings.iter().all(|finding| finding.suppressed),
        "the log carries the reason the author gave"
    );
    assert_eq!(honoured.code, 0);

    let strict = repo.weeder(&["check", "--strict"]);
    let findings = strict.findings();
    assert!(
        findings.iter().all(|finding| finding.level == "error"),
        "under --strict a suppressed block finding is a block finding again"
    );
    assert!(
        findings.iter().all(|finding| finding.suppressed),
        "reported, not honoured: the suppression still travels in the log"
    );
    assert_eq!(strict.code, 2);
}

#[test]
fn a_trailer_on_a_commit_in_the_base_range_counts_and_git_s_own_editmsg_never_does() {
    let repo = Repo::init();
    let base = repo.head();
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.commit(&format!(
        "Split on semicolons

Weed-allow: G1 {REASON}
"
    ));

    let honoured = repo.weeder(&["check", "--base", &base]);
    assert!(honoured.findings().iter().all(|finding| finding.suppressed));
    assert_eq!(
        honoured.code, 0,
        "the trailer rode in on the commit it was written for"
    );

    // The same message still sits in git's COMMIT_EDITMSG. A later, unrelated
    // change must not inherit it.
    repo.write("src/other.ts", &conflicted_parser(None));
    repo.stage_all();
    let later = repo.weeder(&["check"]);
    assert!(
        later.findings().iter().all(|finding| !finding.suppressed),
        "a trailer never outlives the change it was written for"
    );
    assert_eq!(later.code, 2);
}

#[test]
fn an_inline_comment_makes_a_block_finding_a_note_and_strict_gives_it_back() {
    let repo = Repo::init();
    repo.write(
        "src/parser.ts",
        &conflicted_parser(Some(&format!("// weed-allow G1: {REASON}"))),
    );
    repo.stage_all();

    let honoured = repo.weeder(&["check"]);
    let findings = honoured.findings();
    assert_eq!(findings.len(), 3, "one finding per marker");
    assert!(
        findings.iter().all(|finding| finding.suppressed),
        "each marker carries the reason beside it"
    );
    assert!(findings.iter().all(|finding| finding.level == "note"));
    assert_eq!(honoured.code, 0);

    let strict = repo.weeder(&["check", "--strict"]);
    let findings = strict.findings();
    assert_eq!(findings.len(), 3);
    assert!(findings.iter().all(|finding| finding.level == "error"));
    assert_eq!(strict.code, 2);
}

#[test]
fn a_weeder_allow_with_no_reason_is_a_complaint_and_strict_refuses_to_run() {
    let repo = Repo::init();
    repo.write(
        "src/parser.ts",
        &format!(
            "export function parse(input: string): string[] {{\n\
             \x20 // weed-allow G1\n\
             {opener}\n  return input.split(\",\");\n\
             {separator}\n  return input.split(\";\");\n\
             {closer}\n}}\n",
            opener = ours("HEAD"),
            separator = separator(),
            closer = theirs(BRANCH),
        ),
    );
    repo.stage_all();

    let lenient = repo.weeder(&["check"]);
    assert_eq!(lenient.code, 2, "the findings still stand");
    assert_eq!(
        lenient.stderr_lines().len(),
        1,
        "the suppression weeder could not read is said out loud"
    );

    let strict = repo.weeder(&["check", "--strict"]);
    assert_eq!(
        strict.code, 3,
        "under --strict weeder will not judge what it cannot read"
    );
    assert_eq!(strict.stderr_lines().len(), 1, "one line, naming the cause");
}
