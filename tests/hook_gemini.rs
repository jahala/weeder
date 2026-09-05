//! `weed hook gemini` — the Gemini CLI hook event, answered in Gemini's shape.
//!
//! Gemini names the same two moments differently: `BeforeTool` is where a tool
//! call can still be refused, and `AfterAgent` is where a turn tries to end. Its
//! answer is a flat decision rather than Claude's nested one, and `deny` and
//! `block` are both refusals to it. The proof that a real session honours the
//! answer is `scripts/proof/gemini-stop.sh`, which the owner runs.

mod common;

use common::{conflicted_parser, Repo};
use serde_json::{json, Value};

/// The parser once the merge is finished: one side kept, no marker left.
const RESOLVED: &str = "export function parse(input: string): string[] {\n  \
                        return input.split(\";\");\n}\n";

const REFUSED: i32 = 2;
const ALLOWED: i32 = 0;

#[test]
fn a_commit_over_an_index_that_blocks_is_denied_in_geminis_shape() {
    let repo = Repo::init();
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    let run = repo.weed_reading(
        &["hook", "gemini"],
        &tool_event(&repo, "git commit -m 'the merge, half finished'"),
    );

    assert_eq!(
        run.code,
        REFUSED,
        "weed refuses the commit: {}",
        run.output()
    );
    let answer = answer(&run.stdout);
    assert_eq!(
        answer["decision"], "deny",
        "gemini reads a refusal off the decision, not off a nested output:\n{}",
        run.stdout
    );
    assert!(
        answer["hookSpecificOutput"].is_null(),
        "gemini's BeforeTool answer carries no hookSpecificOutput:\n{}",
        run.stdout
    );
    let reason = reason(&answer);
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
        &["hook", "gemini"],
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
    repo.write("src/parser.ts", RESOLVED);
    repo.stage_all();

    for command in [
        "git commit --no-verify -m 'quick'",
        "git commit -n -m 'quick'",
        "git -c core.hooksPath=/dev/null commit -m 'quick'",
    ] {
        let run = repo.weed_reading(&["hook", "gemini"], &tool_event(&repo, command));
        assert_eq!(
            run.code,
            REFUSED,
            "`{command}` walks past the judgement: {}",
            run.output()
        );
        assert_eq!(answer(&run.stdout)["decision"], "deny");
    }
}

#[test]
fn an_after_agent_over_a_tree_that_blocks_is_blocked_and_a_clean_one_is_allowed() {
    let repo = Repo::init();
    repo.write("src/parser.ts", RESOLVED);
    repo.commit("the parser");
    // Never staged: a turn ends over the working tree, whatever the index holds.
    repo.write("src/parser.ts", &conflicted_parser(None));

    let run = repo.weed_reading(&["hook", "gemini"], &after_agent_event(&repo, false));

    assert_eq!(run.code, REFUSED, "weed blocks the stop: {}", run.output());
    let answer = answer(&run.stdout);
    assert_eq!(answer["decision"], "block");
    let reason = reason(&answer);
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
    let allowed = repo.weed_reading(&["hook", "gemini"], &after_agent_event(&repo, false));
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
fn an_after_agent_that_is_already_blocking_blocks_again_over_a_tree_that_still_blocks() {
    let repo = Repo::init();
    repo.write("src/parser.ts", RESOLVED);
    repo.commit("the parser");
    repo.write("src/parser.ts", &conflicted_parser(None));

    let run = repo.weed_reading(&["hook", "gemini"], &after_agent_event(&repo, true));

    assert_eq!(
        run.code,
        REFUSED,
        "a second attempt at done over the same tree is the same answer: {}",
        run.output()
    );
    assert_eq!(answer(&run.stdout)["decision"], "block");
}

#[test]
fn claudes_own_event_names_mean_nothing_to_the_gemini_hook() {
    let repo = Repo::init();
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    for event in [
        json!({"hook_event_name": "PreToolUse", "tool_name": "run_shell_command",
               "tool_input": {"command": "git commit -m 'done'"}}),
        json!({"hook_event_name": "Stop", "stop_hook_active": false}),
        json!({"hook_event_name": "AfterTool", "tool_name": "run_shell_command",
               "tool_input": {"command": "git commit -m 'done'"}}),
        json!({"hook_event_name": "SessionStart", "source": "startup"}),
    ] {
        let mut event = event;
        event["cwd"] = json!(repo.root().display().to_string());
        let run = repo.weed_reading(&["hook", "gemini"], &event.to_string());
        assert_eq!(
            run.code,
            ALLOWED,
            "gemini never sends {} to a hook, so weed has no business answering it: {}",
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
        "git add -A && git commit -m 'half finished'",
        "sh -c \"git commit -m 'half finished'\"",
        "git commit --message='half finished' 2>/dev/null",
    ] {
        let run = repo.weed_reading(&["hook", "gemini"], &tool_event(&repo, command));
        assert_eq!(
            run.code,
            REFUSED,
            "`{command}` runs git commit: {}",
            run.output()
        );
    }

    for command in ["git status", "echo 'git commit -m done'"] {
        let run = repo.weed_reading(&["hook", "gemini"], &tool_event(&repo, command));
        assert_eq!(
            run.code,
            ALLOWED,
            "`{command}` makes no commit: {}",
            run.output()
        );
    }
}

/// The event Gemini CLI writes before it runs a tool.
fn tool_event(repo: &Repo, command: &str) -> String {
    json!({
        "session_id": "a-session",
        "transcript_path": "/dev/null",
        "cwd": repo.root().display().to_string(),
        "hook_event_name": "BeforeTool",
        "timestamp": "2026-09-05T09:00:00.000Z",
        "tool_name": "run_shell_command",
        "tool_input": {"command": command, "description": "commit the work"},
    })
    .to_string()
}

/// The event Gemini CLI writes when the agent loop has finished.
fn after_agent_event(repo: &Repo, active: bool) -> String {
    json!({
        "session_id": "a-session",
        "transcript_path": "/dev/null",
        "cwd": repo.root().display().to_string(),
        "hook_event_name": "AfterAgent",
        "timestamp": "2026-09-05T09:00:00.000Z",
        "prompt": "finish the merge",
        "prompt_response": "done",
        "stop_hook_active": active,
    })
    .to_string()
}

fn answer(stdout: &str) -> Value {
    serde_json::from_str(stdout)
        .unwrap_or_else(|error| panic!("stdout should be gemini's hook JSON: {error}\n{stdout}"))
}

fn reason(answer: &Value) -> String {
    answer["reason"]
        .as_str()
        .unwrap_or_else(|| panic!("a refusal carries its reason:\n{answer}"))
        .to_string()
}
