//! X2, a file outside the scope was touched.
//!
//! A scope is the sentence "this change is about that", written as globs. A
//! file the change reached that the sentence does not cover is work nobody
//! asked for, and it is the quiet way a small change grows. The finding names
//! the blast radius with it: who calls the definitions that just moved.
//!
//! The neighbour is the same edit with no scope named at all. weeder will not
//! invent one, so a run that was given no sentence has nothing to hold the
//! change against.

mod common;

use common::fixture;

/// The file the change was scoped for, and the one it reached anyway.
const INSIDE: &str = "src/report/summary.ts";
const OUTSIDE: &str = "src/wire/client.ts";
/// The file that calls what the out-of-scope file defines.
const CALLER: &str = "src/app/main.ts";
/// The definition that moved under the caller's feet.
const MOVED: &str = "sendPayload";

#[test]
fn x2_fires_at_block_level_on_the_file_outside_the_scope_and_names_its_callers() {
    let repo = fixture("X2", "paths", "fire");
    let run = repo.weeder(&["check"]);

    assert_eq!(
        run.code, 2,
        "a file the change was never scoped for stops it\n{}",
        run.stderr
    );
    let findings = run.findings();
    assert_eq!(findings.len(), 1, "one file outside the scope, one finding");
    let finding = &findings[0];
    assert_eq!(finding.rule, "X2", "the rule is X2");
    assert_eq!(finding.level, "error", "X2 blocks once a scope is named");
    assert_eq!(finding.path, OUTSIDE, "the finding names the file reached");
    assert!(
        finding.message.contains(CALLER) && finding.message.contains(MOVED),
        "the message names what depends on the definition that changed: {}",
        finding.message
    );

    let changed = repo.git(&["diff", "HEAD", "--name-only"]);
    assert!(
        changed.lines().any(|line| line == INSIDE),
        "{INSIDE} must be in the diff, or the silence about it proves nothing"
    );
    assert!(
        !run.paths().contains(&INSIDE.to_string()),
        "the file the change was scoped for is not reported"
    );
}

#[test]
fn x2_stays_silent_where_no_scope_was_named() {
    let repo = fixture("X2", "paths", "silent");
    let changed = repo.git(&["diff", "HEAD", "--name-only"]);
    assert!(
        changed.lines().any(|line| line == OUTSIDE),
        "the neighbour must be in the diff, or the silence proves nothing"
    );

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "with no sentence to hold the change against, weeder will not invent one"
    );
    assert_eq!(run.code, 0, "nothing found, nothing blocked");
}

#[test]
fn the_scope_flag_decides_it_just_as_the_config_does() {
    let repo = fixture("X2", "paths", "silent");

    let inside = repo.weeder(&["check", "--scope", "src/wire/**"]);
    assert_eq!(
        inside.findings(),
        Vec::new(),
        "a file the flag allows is a file the change was for\n{}",
        inside.stderr
    );
    assert_eq!(inside.code, 0);

    let outside = repo.weeder(&["check", "--scope", "src/report/**"]);
    assert_eq!(
        outside.paths(),
        vec![OUTSIDE.to_string()],
        "a file the flag does not allow is reported\n{}",
        outside.stderr
    );
    assert_eq!(outside.code, 2);
    assert!(outside
        .findings()
        .iter()
        .all(|finding| finding.rule == "X2"));
}
