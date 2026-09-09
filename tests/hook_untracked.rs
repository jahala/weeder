//! The stop hook over a tree whose only change is a file git was never told about.
//!
//! Most of what an agent writes is untracked, and `git diff HEAD` lists none of
//! it, so a turn could once end over a production file carrying a stub with
//! weeder saying nothing. The end of a turn is judged over the working tree, and
//! the working tree includes what arrived without git hearing about it.
//!
//! The commit side keeps the older, narrower view, and both faces of it are
//! here: the `git commit` a tool call runs is judged on the index, and so is the
//! pre-commit hook `guard` installs. A commit carries the index, and refusing a
//! commit for a file it does not carry would refuse honest work.
//!
//! Nothing is mocked: the repository is real, the events are the JSON Claude
//! Code writes on a hook's stdin, and the commits are made by git running the
//! hook weeder wrote.

mod common;

use common::Repo;
use serde_json::{json, Value};

/// weeder's contract: 2 is at least one block-level result.
const REFUSED: i32 = 2;
const ALLOWED: i32 = 0;

/// The production file the repository already has, committed, so the only change
/// in the tree is the one the tests are about.
const FORMAT: &str = "export function format(text: string, width: number): string {\n  \
                      return text.padEnd(width);\n}\n";

/// The same file, edited honestly: something to stage while the stub sits beside
/// it untracked.
const FORMAT_TRIMMED: &str = "export function format(text: string, width: number): string {\n  \
                              return text.trim().padEnd(width);\n}\n";

/// A production file an agent wrote and never staged, with the work left
/// unwritten inside it.
const STUB: &str = "export function enqueue(job: string): void {\n  \
                    // TODO: hand the job to the worker pool\n  \
                    throw new Error(\"not implemented\");\n}\n";

/// The same file, finished.
const WRITTEN: &str = "export function enqueue(job: string): void {\n  \
                       queue.push(job);\n}\n\n\
                       const queue: string[] = [];\n";

/// Where the stub lands.
const STUB_PATH: &str = "src/queue.ts";

/// A repository whose one change is the untracked file carrying the stub.
fn only_an_untracked_stub() -> Repo {
    let repo = Repo::init();
    repo.write("src/format.ts", FORMAT);
    repo.commit("the formatter");
    repo.write(STUB_PATH, STUB);
    repo
}

#[test]
fn the_stop_is_blocked_over_a_tree_whose_only_change_is_an_untracked_stub() {
    let repo = only_an_untracked_stub();
    assert_eq!(
        repo.git(&["status", "--porcelain"]).trim(),
        format!("?? {STUB_PATH}"),
        "the whole of the change is a file git has never been told about"
    );

    let run = repo.weeder_reading(&["hook", "claude"], &stop_event(&repo, false));

    assert_eq!(
        run.code,
        REFUSED,
        "weeder blocks the stop: {}",
        run.output()
    );
    let answer: Value = serde_json::from_str(&run.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout should be Claude's hook JSON: {error}\n{}",
            run.stdout
        )
    });
    assert_eq!(answer["decision"], "block");
    let reason = answer["reason"]
        .as_str()
        .expect("a block carries its reason");
    assert!(reason.contains("S1"), "the reason is the table:\n{reason}");
    assert!(
        reason.contains(STUB_PATH),
        "the reason names the file nobody staged:\n{reason}"
    );
    assert!(
        reason.contains("weeder hook refused"),
        "the reason says which gate refused and what to do:\n{reason}"
    );

    repo.write(STUB_PATH, WRITTEN);
    let allowed = repo.weeder_reading(&["hook", "claude"], &stop_event(&repo, false));
    assert_eq!(
        allowed.code,
        ALLOWED,
        "the work written, the turn may end: {}",
        allowed.output()
    );
    assert_eq!(
        allowed.stdout, "",
        "weeder lets a clean stop go without a word"
    );
}

#[test]
fn a_second_attempt_at_done_over_the_same_untracked_stub_is_blocked_again() {
    let repo = only_an_untracked_stub();

    let run = repo.weeder_reading(&["hook", "claude"], &stop_event(&repo, true));

    assert_eq!(
        run.code,
        REFUSED,
        "an unstaged stub does not become honest by being asked twice: {}",
        run.output()
    );
    let answer: Value = serde_json::from_str(&run.stdout).expect("Claude's hook JSON");
    assert_eq!(answer["decision"], "block");
}

#[test]
fn the_commit_the_hook_sees_is_judged_on_the_index_and_not_on_the_untracked_stub() {
    let repo = only_an_untracked_stub();
    repo.write("src/format.ts", FORMAT_TRIMMED);
    repo.git(&["add", "src/format.ts"]);

    let run = repo.weeder_reading(
        &["hook", "claude"],
        &tool_event(&repo, "git commit -m 'trim before padding'"),
    );

    assert_eq!(
        run.code,
        ALLOWED,
        "a commit carries the index, and the index does not carry the stub: {}",
        run.output()
    );
    assert_eq!(run.stdout, "", "and weeder says nothing about it");

    // Staged, the same file is in what the commit would carry, and the same
    // event is denied.
    repo.git(&["add", STUB_PATH]);
    let refused = repo.weeder_reading(
        &["hook", "claude"],
        &tool_event(&repo, "git commit -m 'trim before padding'"),
    );
    assert_eq!(
        refused.code,
        REFUSED,
        "the stub reached the index, so it reached the judgement: {}",
        refused.output()
    );
    let reason = refused.stdout.clone();
    let answer: Value = serde_json::from_str(&reason).expect("Claude's hook JSON");
    let denial = answer["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .expect("a denial carries its reason");
    assert!(
        denial.contains("S1") && denial.contains(STUB_PATH),
        "the reason is the table, naming the file:\n{denial}"
    );
}

#[test]
fn the_pre_commit_hook_guard_installs_still_judges_the_index_alone() {
    let repo = Repo::init();
    repo.weeder(&["guard", "install"]);
    repo.commit("weeder guard installed");
    repo.write("src/format.ts", FORMAT);
    repo.commit("the formatter");
    let before = repo.head();

    repo.write(STUB_PATH, STUB);
    repo.write("src/format.ts", FORMAT_TRIMMED);
    repo.git(&["add", "src/format.ts"]);
    let allowed = repo.try_git(&["commit", "-m", "trim before padding"]);

    assert_eq!(
        allowed.code,
        0,
        "the file git was never told about is in no index, so it is in no commit: {}",
        allowed.output()
    );
    assert_ne!(repo.head(), before, "the honest commit is really there");
    assert_eq!(
        repo.git(&["status", "--porcelain"]).trim(),
        format!("?? {STUB_PATH}"),
        "and the stub is still sitting there, untracked, for the stop to find"
    );

    let stopped = repo.weeder_reading(&["hook", "claude"], &stop_event(&repo, false));
    assert_eq!(
        stopped.code,
        REFUSED,
        "what the commit was right to ignore is what the turn is refused for: {}",
        stopped.output()
    );

    let staged = repo.head();
    repo.git(&["add", STUB_PATH]);
    let refused = repo.try_git(&["commit", "-m", "the queue"]);
    assert_ne!(
        refused.code,
        0,
        "staged, the stub is what the commit carries, and git stops: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(said.contains("S1"), "the hook prints the findings:\n{said}");
    assert!(said.contains(STUB_PATH), "the hook names the file:\n{said}");
    assert_eq!(repo.head(), staged, "no commit was made");
}

/// The event Claude Code writes when the agent has stopped talking.
fn stop_event(repo: &Repo, active: bool) -> String {
    json!({
        "session_id": "a-session",
        "transcript_path": "/dev/null",
        "cwd": repo.root().display().to_string(),
        "hook_event_name": "Stop",
        "stop_hook_active": active,
    })
    .to_string()
}

/// The event Claude Code writes before it runs a Bash tool call.
fn tool_event(repo: &Repo, command: &str) -> String {
    json!({
        "session_id": "a-session",
        "transcript_path": "/dev/null",
        "cwd": repo.root().display().to_string(),
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": command, "description": "commit the work"},
    })
    .to_string()
}
