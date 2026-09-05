//! The harness hook: what an event asks of weed, and how each harness is
//! answered in its own shape.
//!
//! `weed guard` is the law in git, and a git hook sees a commit that reaches
//! git. This is the line before that one: it sees the command an agent is about
//! to run, so it can refuse the commit that asks git to walk past its hooks, and
//! it sees a turn trying to end, which git never hears about at all. An agent
//! cannot declare itself done over a tree weed refuses.
//!
//! Three harnesses, two moments, one judgement. Claude Code calls them
//! `PreToolUse` and `Stop`, Gemini CLI calls them `BeforeTool` and `AfterAgent`,
//! and codex uses Claude's names. What differs is the shape of the answer, and
//! that is all this module holds beyond the reading of the command line itself.

use serde_json::{json, Value};

use crate::core::shell;

/// The agent harnesses weed answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Harness {
    Claude,
    Gemini,
    Codex,
}

/// The events codex takes no decision from. Codex ships a JSON schema for every
/// hook event's output inside its own binary, and these four carry no `decision`
/// and no `permissionDecision`: a hook wired to one of them is heard and cannot
/// refuse. Claude Code and Gemini CLI publish no such machine-readable contract,
/// so weed states nothing about them rather than guessing at another tool's
/// behaviour on its user's behalf.
const CODEX_DEAF: [&str; 4] = ["PreCompact", "PostCompact", "SessionStart", "SubagentStart"];

impl Harness {
    /// What the harness calls itself on the command line, which is also the
    /// word `weed hook` takes.
    pub fn name(self) -> &'static str {
        match self {
            Harness::Claude => "claude",
            Harness::Gemini => "gemini",
            Harness::Codex => "codex",
        }
    }

    /// The event that arrives while a tool call can still be refused.
    fn tool_event(self) -> &'static str {
        match self {
            Harness::Claude | Harness::Codex => "PreToolUse",
            Harness::Gemini => "BeforeTool",
        }
    }

    /// The event that arrives when the turn is trying to end.
    fn stop_event(self) -> &'static str {
        match self {
            Harness::Claude | Harness::Codex => "Stop",
            Harness::Gemini => "AfterAgent",
        }
    }

    fn deaf(self) -> &'static [&'static str] {
        match self {
            Harness::Codex => &CODEX_DEAF,
            Harness::Claude | Harness::Gemini => &[],
        }
    }
}

/// What an event asks of weed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// A commit is about to be made. The directories are what the command line
    /// told git about its own working directory, in the order it named them.
    Commit { directories: Vec<String> },
    /// The commit tells git to walk past the hooks that judge it, spelled the
    /// way the command line spelled it.
    Bypass { flag: String },
    /// The turn is trying to end.
    Stop,
    /// Nothing here weed has anything to say about.
    Pass,
    /// The harness takes no decision from a hook at this event.
    Deaf { event: String },
}

/// Which refusal is being written, and so which shape the harness reads it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The tool call does not run.
    Tool,
    /// The turn does not end.
    Turn,
}

/// What the event asks of weed. An event weed knows nothing about is passed
/// through: a hook that answers where it was not asked is a hook in the way.
pub fn read(harness: Harness, event: &Value) -> Ask {
    let name = event
        .get("hook_event_name")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if name == harness.stop_event() {
        return Ask::Stop;
    }
    if name == harness.tool_event() {
        return tool_call(event.get("tool_input").unwrap_or(&Value::Null));
    }
    if harness.deaf().contains(&name) {
        return Ask::Deaf {
            event: name.to_string(),
        };
    }
    Ask::Pass
}

/// Where the harness says it is working. An event that names no directory
/// leaves weed where it was started from.
pub fn working_directory(event: &Value) -> Option<&str> {
    event.get("cwd").and_then(Value::as_str)
}

/// weed's answer, in the shape the harness reads it in.
pub fn answer(harness: Harness, refusal: Refusal, reason: &str) -> Value {
    match refusal {
        // Every one of the three refuses the end of a turn the same way.
        Refusal::Turn => json!({"decision": "block", "reason": reason}),
        // Gemini reads one flat decision at a tool call too; the other two want
        // the decision under the event it belongs to.
        Refusal::Tool => match harness {
            Harness::Gemini => json!({"decision": "deny", "reason": reason}),
            Harness::Claude | Harness::Codex => json!({
                "hookSpecificOutput": {
                    "hookEventName": harness.tool_event(),
                    "permissionDecision": "deny",
                    "permissionDecisionReason": reason,
                }
            }),
        },
    }
}

/// The one line weed prints at an event its harness takes no decision from. It
/// is not an answer, there is nowhere to put one, so it says where a hook that
/// is meant to refuse belongs instead.
pub fn note(harness: Harness, event: &str) -> String {
    let cli = harness.name();
    format!(
        "{cli} takes no decision from a {event} hook, so weed judged nothing here. wire weed hook {cli} to {} and {}, the events {cli} lets a hook refuse.",
        harness.tool_event(),
        harness.stop_event(),
    )
}

/// The reason a commit over an index that blocks is refused: the table, and then
/// what to do about it.
pub fn commit_refused(table: &str) -> String {
    format!(
        "{table}weed hook refused: the index carries a finding that blocks, and a commit made over it puts the finding in the history. repair what the table names and commit again, or take that change back out of the index.\n"
    )
}

/// The reason a commit that asks git to skip its own hooks is refused. There is
/// no table: nothing was judged, because the command asked for nothing to be.
pub fn bypass_refused(flag: &str) -> String {
    format!(
        "weed hook refused: this commit carries {flag}, which tells git to walk past the hooks that judge it, and a change that needs the judgement skipped is the change the judgement is for. commit without it.\n"
    )
}

/// The reason a turn that ends over a tree that blocks is refused.
pub fn stop_refused(table: &str) -> String {
    format!(
        "{table}weed hook refused: the working tree carries a finding that blocks, and a turn that ends here ends over it. repair what the table names, then say the work is done.\n"
    )
}

/// The reason a run weed could not finish is refused rather than waved through.
/// A harness reads only a refusal or silence from a hook, so a gate that could
/// not judge has to refuse to stay a gate.
pub fn unjudged(reason: &str) -> String {
    format!("weed hook refused: {reason} a gate that could not judge refuses rather than letting the change past.\n")
}

/// What a tool call is asking for: the commands the harness is about to run,
/// read for a `git commit` among them.
fn tool_call(input: &Value) -> Ask {
    let mut directories = Vec::new();
    for command in commands(input) {
        // A command line that walks somewhere first walks git there with it.
        if shell::program(&command) == "cd" {
            if let Some(target) = command
                .iter()
                .skip(1)
                .find(|word| !word.starts_with('-') && *word != "--")
            {
                directories.push(target.clone());
            }
            continue;
        }
        let Some(commit) = commit(&command) else {
            continue;
        };
        if let Some(flag) = commit.bypass {
            return Ask::Bypass { flag };
        }
        directories.extend(commit.directories);
        return Ask::Commit { directories };
    }
    Ask::Pass
}

/// The commands a tool call will run. A harness writes them as the text of a
/// command line, or, where it already broke them into words, as those words.
fn commands(input: &Value) -> Vec<Vec<String>> {
    match input.get("command") {
        Some(Value::String(line)) => shell::commands(line),
        Some(Value::Array(words)) => {
            let words: Vec<String> = words
                .iter()
                .filter_map(|word| word.as_str().map(str::to_string))
                .collect();
            shell::commands_of(&words)
        }
        _ => Vec::new(),
    }
}

/// A `git commit` a command asks for, and what it asks of git along the way.
struct Commit {
    directories: Vec<String>,
    bypass: Option<String>,
}

/// git's own options that take the word after them as their value. They come
/// before the subcommand, and a value must never be read as an option.
const GIT_VALUE_OPTIONS: [&str; 8] = [
    "-C",
    "-c",
    "--config-env",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--exec-path",
    "--attr-source",
];
/// The configuration key that decides where git looks for its hooks. Setting it
/// for one command is `--no-verify` by another name.
const HOOKS_PATH_KEY: &str = "core.hookspath";
/// The same key as a refusal names it back to the person who wrote it.
const HOOKS_PATH_FLAG: &str = "core.hooksPath";
/// `git commit`'s short options that take a value: the rest of the cluster if
/// there is any, and otherwise the word after it.
const COMMIT_VALUE_SHORT: [char; 5] = ['F', 'm', 'c', 'C', 't'];
/// `git commit`'s short options that take a value only where it is written
/// against them, so the rest of the cluster is theirs either way.
const COMMIT_ATTACHED_SHORT: [char; 2] = ['S', 'u'];
/// `git commit`'s long options that take the word after them as their value.
const COMMIT_VALUE_LONG: [&str; 14] = [
    "file",
    "message",
    "reuse-message",
    "reedit-message",
    "fixup",
    "squash",
    "author",
    "date",
    "template",
    "cleanup",
    "gpg-sign",
    "untracked-files",
    "pathspec-from-file",
    "trailer",
];

/// The commit this command makes, or `None` where it makes none. `git` is the
/// program, `commit` is the subcommand, and everything between them is git's own
/// options, which is where a command can move git somewhere else, or take its
/// hooks away without ever writing `--no-verify`.
fn commit(command: &[String]) -> Option<Commit> {
    if shell::program(command) != "git" {
        return None;
    }

    let mut directories = Vec::new();
    let mut bypass = None;
    let mut words = command.iter().skip(1);
    let subcommand = loop {
        let word = words.next()?;
        if !word.starts_with('-') {
            break word;
        }
        let value = match word.split_once('=') {
            Some((name, attached)) if GIT_VALUE_OPTIONS.contains(&name) => Some(attached),
            _ if GIT_VALUE_OPTIONS.contains(&word.as_str()) => words.next().map(String::as_str),
            // `-C<path>` and `-c<name>=<value>` may be written against the flag.
            _ if word.len() > 2
                && matches!(word.get(..2), Some(flag) if GIT_VALUE_OPTIONS.contains(&flag)) =>
            {
                word.get(2..)
            }
            _ => None,
        };
        let Some(value) = value else {
            continue;
        };
        if word.starts_with("-C") {
            directories.push(value.to_string());
        } else if names_hooks_path(value) {
            bypass = Some(format!("{} {HOOKS_PATH_FLAG}", flag_of(word)));
        }
    };
    if subcommand != "commit" {
        return None;
    }

    let mut options = true;
    let mut value_follows = false;
    for word in words {
        if value_follows {
            value_follows = false;
            continue;
        }
        if !options {
            continue;
        }
        if word == "--" {
            options = false;
            continue;
        }
        if let Some(long) = word.strip_prefix("--") {
            if long.contains('=') {
                continue;
            }
            match long {
                "no-verify" => bypass = Some("--no-verify".to_string()),
                // git lets a later --verify take the bypass back.
                "verify" => bypass = None,
                _ if COMMIT_VALUE_LONG.contains(&long) => value_follows = true,
                _ => {}
            }
            continue;
        }
        let Some(cluster) = word.strip_prefix('-') else {
            continue;
        };
        for (position, character) in cluster.char_indices() {
            if COMMIT_VALUE_SHORT.contains(&character) {
                // The value is the rest of the cluster, and where there is no
                // rest, the word after it.
                value_follows = position + character.len_utf8() == cluster.len();
                break;
            }
            if COMMIT_ATTACHED_SHORT.contains(&character) {
                break;
            }
            if character == 'n' {
                bypass = Some("-n".to_string());
            }
        }
    }

    Some(Commit {
        directories,
        bypass,
    })
}

/// The flag a word carries, without the value written against it, so a refusal
/// names what the command wrote rather than what it wrote it as.
fn flag_of(word: &str) -> &str {
    match word.split_once('=') {
        Some((flag, _)) if flag.starts_with("--") => flag,
        _ if word.starts_with("--") => word,
        _ => word.get(..2).unwrap_or(word),
    }
}

/// Whether a git configuration setting names where git looks for its hooks.
/// Section and key are case-insensitive to git, so they are here too.
fn names_hooks_path(setting: &str) -> bool {
    setting
        .split('=')
        .any(|part| part.to_ascii_lowercase() == HOOKS_PATH_KEY)
}
