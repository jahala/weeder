//! `weed hook codex`, the Codex CLI hook event, and the events it cannot refuse.
//!
//! Codex ships a JSON schema for every hook event's input and output inside its
//! own binary, and those schemas say plainly which events carry a decision. Two
//! of them are where weed stands: `PreToolUse` refuses a tool call, `Stop`
//! refuses the end of a turn. Four carry no decision at all, and a hook wired to
//! one of those can be heard but cannot refuse, so weed says one line and
//! judges nothing, rather than looking like a gate that is not there.
//!
//! `docs/proof-2026-09.md` is where that reading is written down. This file
//! reads it back and holds weed to it, in both directions: every event the doc
//! calls deaf gets the note, every event it does not stays silent.

mod common;

use std::path::{Path, PathBuf};

use common::{conflicted_parser, Repo};
use serde_json::{json, Value};

const REFUSED: i32 = 2;
const ALLOWED: i32 = 0;

/// The heading in the proof document that owns codex's hook contract.
const SECTION: &str = "## codex: what a hook may refuse";
/// What the doc's middle column says of an event that takes no decision.
const NO_DECISION: &str = "no decision";
/// The two events weed stands at, and so the two the rest of the contract is
/// measured against.
const STANDING: [&str; 2] = ["PreToolUse", "Stop"];

#[test]
fn a_commit_over_an_index_that_blocks_is_denied_in_codexs_shape() {
    let repo = Repo::init();
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    let run = repo.weed_reading(
        &["hook", "codex"],
        &tool_event(
            &repo,
            json!(["bash", "-lc", "git commit -m 'half finished'"]),
        ),
    );

    assert_eq!(
        run.code,
        REFUSED,
        "weed refuses the commit: {}",
        run.output()
    );
    let answer = answer(&run.stdout);
    let specific = &answer["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], "PreToolUse");
    assert_eq!(
        specific["permissionDecision"], "deny",
        "deny is the only permission decision codex accepts from a PreToolUse hook:\n{}",
        run.stdout
    );
    let reason = specific["permissionDecisionReason"]
        .as_str()
        .expect("codex refuses a deny that carries no reason");
    assert!(!reason.trim().is_empty(), "and the reason is not empty");
    assert!(reason.contains("G1"), "the reason is the table:\n{reason}");
    assert!(
        reason.contains("src/parser.ts"),
        "the reason names the file:\n{reason}"
    );

    // Codex hands a shell call its argv, so weed reads the script out of it.
    let string_form = repo.weed_reading(
        &["hook", "codex"],
        &tool_event(&repo, json!("git commit -m 'half finished'")),
    );
    assert_eq!(
        string_form.code,
        REFUSED,
        "a command written as one string is the same commit: {}",
        string_form.output()
    );
}

#[test]
fn a_commit_over_a_clean_index_is_not_denied() {
    let repo = Repo::init();
    repo.write(
        "src/parser.ts",
        "export function parse(input: string): string[] {\n  return input.split(\";\");\n}\n",
    );
    repo.stage_all();

    let run = repo.weed_reading(
        &["hook", "codex"],
        &tool_event(&repo, json!(["bash", "-lc", "git commit -m 'finished'"])),
    );

    assert_eq!(run.code, ALLOWED, "a clean index commits: {}", run.output());
    assert_eq!(
        run.stdout, "",
        "weed says nothing where it has nothing to say"
    );
}

#[test]
fn a_stop_over_a_tree_that_blocks_is_blocked_in_codexs_shape() {
    let repo = Repo::init();
    repo.write(
        "src/parser.ts",
        "export function parse(input: string): string[] {\n  return input.split(\";\");\n}\n",
    );
    repo.commit("the parser");
    repo.write("src/parser.ts", &conflicted_parser(None));

    let run = repo.weed_reading(&["hook", "codex"], &stop_event(&repo));

    assert_eq!(run.code, REFUSED, "weed blocks the stop: {}", run.output());
    let answer = answer(&run.stdout);
    assert_eq!(answer["decision"], "block");
    let reason = answer["reason"]
        .as_str()
        .expect("codex refuses a block that carries no reason");
    assert!(!reason.trim().is_empty(), "and the reason is not empty");
    assert!(reason.contains("G1"), "the reason is the table:\n{reason}");
}

#[test]
fn the_events_codex_takes_no_decision_from_get_one_line_and_no_answer() {
    let repo = Repo::init();
    // Red, so silence here is weed declining to answer rather than weed
    // finding nothing to say.
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    let doc = read_proof();
    let contract = contract(&doc);
    let deaf: Vec<&String> = contract
        .iter()
        .filter(|(_, deaf)| *deaf)
        .map(|(event, _)| event)
        .collect();
    assert!(
        !deaf.is_empty(),
        "{SECTION} names no event codex takes no decision from"
    );
    // The document quotes the line once, for one of those events; every other
    // one is that line with its own name in it.
    let recorded = quoted(&doc);
    let example = deaf
        .iter()
        .find(|event| recorded.contains(event.as_str()))
        .unwrap_or_else(|| {
            panic!("the line `{SECTION}` quotes names no event it calls deaf:\n{recorded}")
        })
        .to_string();

    for (event, deaf) in &contract {
        // The two weed stands at answer for themselves, above.
        if STANDING.contains(&event.as_str()) {
            continue;
        }
        let run = repo.weed_reading(&["hook", "codex"], &event_named(&repo, event));
        assert_eq!(
            run.stdout,
            "",
            "{event} is not one of the two places weed stands, so it gets no answer: {}",
            run.output()
        );
        assert_eq!(
            run.code,
            ALLOWED,
            "{event} is passed through: {}",
            run.output()
        );
        if *deaf {
            let note = run.stderr.trim_end();
            assert_eq!(
                note.lines().count(),
                1,
                "{event} is answered with one line, not a lecture:\n{note}"
            );
            assert!(
                note.contains(event),
                "the line names the event it is about:\n{note}"
            );
            assert_eq!(
                note,
                recorded.replace(&example, event),
                "the line weed prints at {event} is not the line docs/proof-2026-09.md records"
            );
        } else {
            assert_eq!(
                run.stderr, "",
                "{event} carries a decision, so weed has nothing to explain"
            );
        }
    }
}

#[test]
fn the_two_events_weed_stands_at_are_the_ones_the_document_says_carry_a_decision() {
    let doc = read_proof();
    let contract = contract(&doc);

    for standing in STANDING {
        let row = contract
            .iter()
            .find(|(event, _)| event == standing)
            .unwrap_or_else(|| panic!("{SECTION} has no row for {standing}"));
        assert!(
            !row.1,
            "weed refuses at {standing}, so the document may not call it deaf"
        );
    }
}

/// A codex PreToolUse event, with the tool input codex writes for a shell call.
fn tool_event(repo: &Repo, command: Value) -> String {
    json!({
        "session_id": "a-session",
        "turn_id": "a-turn",
        "transcript_path": null,
        "cwd": repo.root().display().to_string(),
        "model": "a-model",
        "permission_mode": "default",
        "hook_event_name": "PreToolUse",
        "tool_name": "shell",
        "tool_use_id": "a-call",
        "tool_input": {"command": command, "workdir": repo.root().display().to_string()},
    })
    .to_string()
}

fn stop_event(repo: &Repo) -> String {
    json!({
        "session_id": "a-session",
        "turn_id": "a-turn",
        "transcript_path": null,
        "cwd": repo.root().display().to_string(),
        "model": "a-model",
        "permission_mode": "default",
        "hook_event_name": "Stop",
        "last_assistant_message": "done",
        "stop_hook_active": false,
    })
    .to_string()
}

/// An event of the named kind, carrying a commit weed would refuse if it were
/// standing there. Every event but `PreToolUse` and `Stop` must let it past.
fn event_named(repo: &Repo, event: &str) -> String {
    json!({
        "session_id": "a-session",
        "turn_id": "a-turn",
        "transcript_path": null,
        "cwd": repo.root().display().to_string(),
        "model": "a-model",
        "permission_mode": "default",
        "hook_event_name": event,
        "tool_name": "shell",
        "tool_use_id": "a-call",
        "tool_input": {"command": ["bash", "-lc", "git commit -m 'half finished'"]},
        "stop_hook_active": false,
        "trigger": "auto",
        "source": "startup",
    })
    .to_string()
}

/// Codex's hook contract as the proof document records it: every event, and
/// whether codex takes a decision from a hook there.
fn contract(doc: &str) -> Vec<(String, bool)> {
    let section = doc
        .split_once(SECTION)
        .unwrap_or_else(|| panic!("docs/proof-2026-09.md has no `{SECTION}` section"))
        .1;
    let section = section.split("\n## ").next().unwrap_or(section);

    let mut rows = Vec::new();
    for line in section.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().trim_matches('`').trim())
            .collect();
        if cells.len() < 2 || cells[0].starts_with("---") || cells[0] == "event" {
            continue;
        }
        rows.push((cells[0].to_string(), cells[1] == NO_DECISION));
    }
    assert!(
        rows.len() > 1,
        "`{SECTION}` carries no table of codex's events"
    );
    rows
}

/// The one line the section quotes, as weed would have to print it.
fn quoted(doc: &str) -> String {
    let section = doc
        .split_once(SECTION)
        .unwrap_or_else(|| panic!("docs/proof-2026-09.md has no `{SECTION}` section"))
        .1;
    let section = section.split("\n## ").next().unwrap_or(section);
    let block = section
        .split_once("\n```\n")
        .unwrap_or_else(|| panic!("`{SECTION}` quotes no line weed prints"))
        .1;
    block
        .split_once("\n```")
        .map(|(quoted, _)| quoted.trim().to_string())
        .unwrap_or_else(|| panic!("`{SECTION}` leaves its quoted line unclosed"))
}

fn read_proof() -> String {
    let path = repository_root().join("docs/proof-2026-09.md");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()))
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn answer(stdout: &str) -> Value {
    serde_json::from_str(stdout)
        .unwrap_or_else(|error| panic!("stdout should be codex's hook JSON: {error}\n{stdout}"))
}
