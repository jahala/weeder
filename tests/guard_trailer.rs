//! The trailer stage: a guardrail change a person means, allowed by what that
//! person wrote on the commit.
//!
//! Nothing here calls weeder directly once the hooks are in. The test runs
//! `git commit` and reads what git did, because the question is whether a person
//! can land a change weeder blocks on without going around the gate, and only
//! git can answer it.
//!
//! pre-commit runs before a message exists, so it can honour nothing and says
//! so: it names the exact trailer that would allow each rule and leaves the
//! verdict to commit-msg, which reads the message. An inline marker is the
//! agent's own line and is reported at its own level in every hook.
//!
//! The deferral is only sound while there is a stage to defer to, and the last
//! case here takes that stage away and holds pre-commit to refusing on its own.

mod common;

/// A hook git cannot run is a hook whose execute bit is off, and only unix has
/// one: the test that takes it away is gated below and Windows has no such shape.
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use common::Repo;

/// The rule a guardrail change fires, and the one every case here is about.
const RULE: &str = "C1";
/// A guardrail path the stem plants and weeder refuses to see changed in silence:
/// a hook of the repository's own, beside the four weeder wrote.
const GUARDRAIL: &str = ".githooks/post-commit";
/// What the repository wants in that hook. It is a shell script, which is why
/// the inline case below has somewhere to write a marker.
const PLANTED: &str = "#!/bin/sh\n# the stem's own hook, planted beside weeder's\nexit 0\n";

/// The reason a person gives for the change. The log has to carry it back.
const REASON: &str = "the stem plants its own hook, and the owner asked for it";

/// A repository with the four hooks installed and already committed.
///
/// `install` writes them into the working tree so a clone gets them, and a hook
/// is a guardrail path; C1 knows the bundle weeder writes byte for byte, so the
/// commit that first carries all four goes through the four hooks it installs.
fn guarded() -> Repo {
    let repo = Repo::init();
    let run = repo.weeder(&["guard", "install"]);
    assert_eq!(run.code, 0, "install runs clean\n{}", run.stderr);
    repo.commit("weeder guard installed");
    repo
}

/// The guardrail change staged and nothing else, ready to be committed.
fn staged(repo: &Repo, contents: &str) {
    repo.write(GUARDRAIL, contents);
    repo.stage_all();
}

/// A message with its trailer, as a person writes one.
fn allowing(rule: &str, reason: &str) -> String {
    format!("plant the stem's own hook\n\nWeeder-allow: {rule} {reason}\n")
}

#[test]
fn pre_commit_names_the_trailer_that_would_allow_the_change_and_the_stage_that_reads_it() {
    let repo = guarded();
    let before = repo.head();
    staged(&repo, PLANTED);

    let refused = repo.try_git(&["commit", "-m", "plant the stem's own hook"]);

    assert_ne!(
        refused.code,
        0,
        "a guardrail change nobody allowed does not commit: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(said.contains(RULE), "the hooks print the finding:\n{said}");
    assert!(said.contains(GUARDRAIL), "the hooks name the file:\n{said}");
    assert!(
        said.contains(&format!("Weeder-allow: {RULE} <reason>")),
        "pre-commit names the exact trailer that would allow it:\n{said}"
    );
    assert!(
        said.contains("commit-msg"),
        "pre-commit says which stage reads that trailer:\n{said}"
    );
    assert_eq!(
        repo.head(),
        before,
        "no commit was made, so the guardrail change never reached the history"
    );
}

#[test]
fn the_same_index_commits_when_the_message_carries_the_trailer() {
    let repo = guarded();
    let before = repo.head();
    staged(&repo, PLANTED);

    assert_ne!(
        repo.try_git(&["commit", "-m", "plant the stem's own hook"])
            .code,
        0,
        "the index blocks before the allowance is written"
    );

    let allowed = repo.try_git(&["commit", "-m", &allowing(RULE, REASON)]);

    assert_eq!(
        allowed.code,
        0,
        "the same index commits once a person allows it on the commit: {}",
        allowed.output()
    );
    assert_ne!(repo.head(), before, "the commit is really there");
    assert_eq!(
        repo.git(&["show", "--stat", "--format=", "--name-only", "HEAD"])
            .trim(),
        GUARDRAIL,
        "the commit carries the guardrail change the trailer allowed"
    );
}

#[test]
fn the_log_of_the_allowed_commit_carries_the_finding_as_a_note_with_its_reason() {
    let repo = guarded();
    staged(&repo, PLANTED);
    let landed = repo.try_git(&["commit", "-m", &allowing(RULE, REASON)]);
    assert_eq!(landed.code, 0, "the commit lands: {}", landed.output());

    let run = repo.weeder(&["check", "--base", "HEAD~1", "--format", "sarif"]);

    assert_eq!(
        run.code, 0,
        "an allowed finding stops nobody\n{}",
        run.stderr
    );
    let finding = run
        .findings()
        .into_iter()
        .find(|finding| finding.rule == RULE)
        .unwrap_or_else(|| panic!("the log still carries the finding:\n{}", run.stdout));
    assert_eq!(
        finding.level, "note",
        "an allowance turns the finding into a note that stops nobody:\n{}",
        run.stdout
    );
    assert!(
        finding.suppressed,
        "the log records the allowance as a suppression:\n{}",
        run.stdout
    );
    assert!(
        run.stdout.contains(REASON),
        "the log carries the reason the person gave:\n{}",
        run.stdout
    );
}

#[test]
fn a_trailer_with_no_reason_allows_nothing() {
    let repo = guarded();
    let before = repo.head();
    staged(&repo, PLANTED);

    let refused = repo.try_git(&[
        "commit",
        "-m",
        &format!("plant the stem's own hook\n\nWeeder-allow: {RULE}\n"),
    ]);

    assert_ne!(
        refused.code,
        0,
        "an allowance with nothing said for it is not an allowance: {}",
        refused.output()
    );
    assert_eq!(repo.head(), before, "no commit was made");
}

#[test]
fn a_trailer_for_another_rule_allows_nothing() {
    let repo = guarded();
    let before = repo.head();
    staged(&repo, PLANTED);

    let refused = repo.try_git(&[
        "commit",
        "-m",
        &allowing(
            "G1",
            "a conflict marker somebody meant to leave, which this is not",
        ),
    ]);

    assert_ne!(
        refused.code,
        0,
        "an allowance names the rule it allows, and this one names another: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(
        said.contains(RULE),
        "the refusal still names the rule that blocks:\n{said}"
    );
    assert_eq!(repo.head(), before, "no commit was made");
}

/// The stage that cannot run is a file git skips for its mode. Windows keeps no
/// mode and git for Windows runs any hook file it finds, so there is no way to
/// put a repository into this state there.
#[cfg(unix)]
#[test]
fn pre_commit_refuses_on_its_own_when_the_stage_it_defers_to_cannot_run() {
    let repo = guarded();
    let before = repo.head();
    // git walks past a hook it cannot run without a word, so this is the shape
    // the deferral has to survive: the file is there and the stage is not.
    let stage = repo.root().join(".githooks/commit-msg");
    std::fs::set_permissions(&stage, std::fs::Permissions::from_mode(0o644))
        .expect("the hook's mode is the test's to change");
    staged(&repo, PLANTED);

    let refused = repo.try_git(&["commit", "-m", &allowing(RULE, REASON)]);

    assert_ne!(
        refused.code,
        0,
        "a gate that hands its verdict to a hook nobody runs is not a gate: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(
        said.contains("weeder guard refused"),
        "pre-commit says it refused, rather than passing the question on:\n{said}"
    );
    assert_eq!(repo.head(), before, "no commit was made");
}

#[test]
fn an_inline_marker_allows_nothing_because_the_line_is_the_agents_to_write() {
    let repo = guarded();
    let before = repo.head();
    staged(
        &repo,
        &format!("#!/bin/sh\n# weeder-allow {RULE}: {REASON}\nexit 0\n"),
    );

    let refused = repo.try_git(&["commit", "-m", "plant the stem's own hook"]);

    assert_ne!(
        refused.code,
        0,
        "a marker on the line is the agent's own and is reported, never honoured: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(
        said.contains(RULE),
        "the finding is reported at its own level:\n{said}"
    );
    assert_eq!(repo.head(), before, "no commit was made");
}
