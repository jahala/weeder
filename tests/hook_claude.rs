//! `weed hook claude` — the Claude Code hook event, answered in Claude's shape.
//!
//! Nothing here is mocked: the events are the JSON Claude Code writes on a
//! hook's stdin, the repository is real, and what the tests read is the JSON
//! weed writes back and the code it leaves with. The proof that a real session
//! honours that answer is `scripts/proof/claude-stop.sh`, cited by the loop.

mod common;

use common::{conflicted_parser, Repo};
use serde_json::{json, Value};

/// The parser once the merge is finished: one side kept, no marker left.
const RESOLVED: &str = "export function parse(input: string): string[] {\n  \
                        return input.split(\";\");\n}\n";

/// weed's contract: 2 is at least one block-level result.
const REFUSED: i32 = 2;
const ALLOWED: i32 = 0;

#[test]
fn a_commit_over_an_index_that_blocks_is_denied_with_the_findings_as_the_reason() {
    let repo = Repo::init();
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    let run = repo.weed_reading(
        &["hook", "claude"],
        &tool_event(&repo, "git commit -m 'the merge, half finished'"),
    );

    assert_eq!(
        run.code,
        REFUSED,
        "weed refuses the commit: {}",
        run.output()
    );
    let answer: Value = serde_json::from_str(&run.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout should be Claude's hook JSON: {error}\n{}",
            run.stdout
        )
    });
    let specific = &answer["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], "PreToolUse");
    assert_eq!(specific["permissionDecision"], "deny");
    let reason = specific["permissionDecisionReason"]
        .as_str()
        .expect("a denial carries its reason");
    assert!(reason.contains("G1"), "the reason is the table:\n{reason}");
    assert!(
        reason.contains("src/parser.ts"),
        "the reason names the file:\n{reason}"
    );
    assert!(
        reason.contains("weed hook refused"),
        "the reason says which gate refused and what to do:\n{reason}"
    );
}

#[test]
fn a_commit_over_a_clean_index_is_not_denied() {
    let repo = Repo::init();
    repo.write("src/parser.ts", RESOLVED);
    repo.stage_all();

    let run = repo.weed_reading(
        &["hook", "claude"],
        &tool_event(&repo, "git commit -m 'the merge, finished'"),
    );

    assert_eq!(run.code, ALLOWED, "a clean index commits: {}", run.output());
    assert_eq!(
        run.stdout, "",
        "weed says nothing where it has nothing to say"
    );
}

#[test]
fn a_commit_that_tells_git_to_skip_its_hooks_is_denied_however_it_is_spelled() {
    let repo = Repo::init();
    // Clean: the bypass is refused for being a bypass, not for what it carries.
    repo.write("src/parser.ts", RESOLVED);
    repo.stage_all();

    for command in [
        "git commit --no-verify -m 'quick'",
        "git commit -n -m 'quick'",
        "git commit -nm 'quick'",
        "git commit -am 'quick' --no-verify",
        "git -c core.hooksPath=/dev/null commit -m 'quick'",
        // git reads the last of the pair, and so does weed.
        "git commit --verify --no-verify -m 'quick'",
    ] {
        let run = repo.weed_reading(&["hook", "claude"], &tool_event(&repo, command));
        assert_eq!(
            run.code,
            REFUSED,
            "`{command}` walks past the judgement: {}",
            run.output()
        );
        let reason = deny_reason(&run.stdout);
        assert!(
            reason.contains("weed hook refused"),
            "`{command}` is refused in weed's voice:\n{reason}"
        );
    }

    for command in [
        // The bypass is what was refused, not the commit.
        "git commit -m 'quick'",
        // A bypass git itself takes back is no bypass.
        "git commit --no-verify --verify -m 'quick'",
        // The message is the word `--no-verify`, and a value is not a flag.
        "git commit --message --no-verify",
    ] {
        let run = repo.weed_reading(&["hook", "claude"], &tool_event(&repo, command));
        assert_eq!(
            run.code,
            ALLOWED,
            "`{command}` asks git for nothing weed refuses: {}",
            run.output()
        );
    }
}

#[test]
fn a_stop_over_a_tree_that_blocks_is_blocked_and_a_clean_one_is_allowed() {
    let repo = Repo::init();
    repo.write("src/parser.ts", RESOLVED);
    repo.commit("the parser");
    // Never staged: a turn ends over the working tree, whatever the index holds.
    repo.write("src/parser.ts", &conflicted_parser(None));

    let run = repo.weed_reading(&["hook", "claude"], &stop_event(&repo, false));

    assert_eq!(run.code, REFUSED, "weed blocks the stop: {}", run.output());
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
    assert!(reason.contains("G1"), "the reason is the table:\n{reason}");
    assert!(
        reason.contains("src/parser.ts"),
        "the reason names the file:\n{reason}"
    );
    assert!(
        reason.contains("weed hook refused"),
        "the reason says which gate refused and what to do:\n{reason}"
    );

    repo.write("src/parser.ts", RESOLVED);
    let allowed = repo.weed_reading(&["hook", "claude"], &stop_event(&repo, false));
    assert_eq!(
        allowed.code,
        ALLOWED,
        "an honest tree may declare itself done: {}",
        allowed.output()
    );
    assert_eq!(
        allowed.stdout, "",
        "weed lets a clean stop go without a word"
    );
}

#[test]
fn a_stop_that_is_already_blocking_blocks_again_over_a_tree_that_still_blocks() {
    let repo = Repo::init();
    repo.write("src/parser.ts", RESOLVED);
    repo.commit("the parser");
    repo.write("src/parser.ts", &conflicted_parser(None));

    let run = repo.weed_reading(&["hook", "claude"], &stop_event(&repo, true));

    assert_eq!(
        run.code,
        REFUSED,
        "a second attempt at done over the same tree is the same answer: {}",
        run.output()
    );
    let answer: Value = serde_json::from_str(&run.stdout).expect("Claude's hook JSON");
    assert_eq!(answer["decision"], "block");
}

#[test]
fn every_other_event_passes_through() {
    let repo = Repo::init();
    // Red, so an event weed passes through is passed through on its own merits.
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    let elsewhere = [
        json!({"hook_event_name": "PostToolUse", "tool_name": "Bash",
               "tool_input": {"command": "git commit -m 'done'"},
               "tool_response": {"stdout": ""}}),
        json!({"hook_event_name": "UserPromptSubmit", "prompt": "commit this"}),
        json!({"hook_event_name": "SubagentStop", "stop_hook_active": false}),
        json!({"hook_event_name": "PreToolUse", "tool_name": "Edit",
               "tool_input": {"file_path": "src/parser.ts", "old_string": "a", "new_string": "b"}}),
    ];
    for event in elsewhere {
        let mut event = event;
        event["cwd"] = json!(repo.root().display().to_string());
        let run = repo.weed_reading(&["hook", "claude"], &event.to_string());
        assert_eq!(
            run.code,
            ALLOWED,
            "weed has no business at {}: {}",
            event["hook_event_name"],
            run.output()
        );
        assert_eq!(
            run.output(),
            "",
            "weed says nothing at {}",
            event["hook_event_name"]
        );
    }
}

#[test]
fn the_commit_is_found_wherever_the_command_line_hides_it() {
    let repo = Repo::init();
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    for command in [
        "git commit -m 'the merge, half finished'",
        "git add -A && git commit -m 'the merge, half finished'",
        "bash -lc \"git commit -m 'the merge, half finished'\"",
        "git status; git commit --message='half finished'",
        "git commit -m 'half finished' > /tmp/commit.log 2>&1",
        "GIT_AUTHOR_NAME=someone git commit -m 'half finished'",
        "bash -lc \"sh -c \\\"git commit -m half-finished\\\"\"",
        // The commit may still run, so weed judges as if it will.
        "true || git commit -m 'half finished'",
        "git commit --amend --no-edit",
    ] {
        let run = repo.weed_reading(&["hook", "claude"], &tool_event(&repo, command));
        assert_eq!(
            run.code,
            REFUSED,
            "`{command}` runs git commit: {}",
            run.output()
        );
    }
}

#[test]
fn a_command_that_only_talks_about_committing_is_left_alone() {
    let repo = Repo::init();
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    for command in [
        "git status",
        "echo 'git commit -m fixed'",
        "grep -r \"git commit\" docs/",
        "git log --format='git commit'",
        "cat <<EOF > notes.md\ngit commit -m 'how to commit'\nEOF",
        "git commit-tree $(git write-tree) -p HEAD -m 'plumbing'",
        "git stash",
    ] {
        let run = repo.weed_reading(&["hook", "claude"], &tool_event(&repo, command));
        assert_eq!(
            run.code,
            ALLOWED,
            "`{command}` makes no commit, and weed judges what runs: {}",
            run.output()
        );
    }
}

#[test]
fn the_commit_is_judged_in_the_repository_the_command_names() {
    let outer = Repo::init();
    outer.write("src/parser.ts", RESOLVED);
    outer.stage_all();

    // A second repository inside the first, and it is the red one.
    let inner = outer.root().join("vendor/inner");
    std::fs::create_dir_all(&inner).expect("a directory for the inner repository");
    for arguments in [
        vec!["init", "--initial-branch=main"],
        vec!["commit", "--allow-empty", "-m", "the repository begins"],
    ] {
        let run = common::git_in(&inner, &arguments);
        assert_eq!(run.code, 0, "git {}: {}", arguments.join(" "), run.output());
    }
    std::fs::create_dir_all(inner.join("src")).expect("a directory for the file");
    std::fs::write(inner.join("src/parser.ts"), conflicted_parser(None))
        .expect("the inner file should be writable");
    let staged = common::git_in(&inner, &["add", "-A"]);
    assert_eq!(staged.code, 0, "the inner index: {}", staged.output());

    let run = outer.weed_reading(
        &["hook", "claude"],
        &tool_event(&outer, "git -C vendor/inner commit -m 'half finished'"),
    );

    assert_eq!(
        run.code,
        REFUSED,
        "the commit lands in the inner repository, so that is the index judged: {}",
        run.output()
    );
    let reason = deny_reason(&run.stdout);
    assert!(
        reason.contains("src/parser.ts"),
        "the inner repository's finding is the reason:\n{reason}"
    );
}

#[test]
fn a_turn_that_ends_outside_a_repository_is_not_blocked() {
    let elsewhere = tempfile::TempDir::new().expect("a directory that is not a repository");
    let event = json!({
        "session_id": "a-session",
        "transcript_path": "/dev/null",
        "cwd": elsewhere.path().display().to_string(),
        "hook_event_name": "Stop",
        "stop_hook_active": false,
    })
    .to_string();

    let run = common::weed_reading_in(elsewhere.path(), &["hook", "claude"], &event);

    assert_eq!(
        run.code,
        ALLOWED,
        "there is no diff to judge where there is no repository, and weed does not hold a turn hostage to that: {}",
        run.output()
    );
    assert_eq!(run.output(), "", "and it says nothing about it");
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

fn deny_reason(stdout: &str) -> String {
    let answer: Value = serde_json::from_str(stdout)
        .unwrap_or_else(|error| panic!("stdout should be Claude's hook JSON: {error}\n{stdout}"));
    answer["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap_or_else(|| panic!("a denial carries its reason:\n{stdout}"))
        .to_string()
}
