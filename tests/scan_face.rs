//! `weeder scan`, the tree judged.
//!
//! The claim this file holds up is that a scan is about the state a repository
//! is in and not about a change: the same tree answers the same way whether the
//! work is untracked, staged or committed. It also holds the face's contract ,
//! 0 with warnings, 3 when weeder could not run, SARIF on a pipe and a table on a
//! terminal, because a consumer reads the exit code before it reads anything
//! else.

mod common;

use std::process::Command;

use common::{weeder_in, Repo, Run};
use serde_json::Value;
use tempfile::TempDir;
use weeder::core::catalogue::{self, Face};
use weeder::core::finding::Level;

/// Three environments a run must not be able to tell apart. Turkish is here
/// because it is the locale that breaks case folding done the naive way: its
/// `i` does not upper-case to `I`, and `--rules r1` is folded before it is
/// looked up. The time zones are far from UTC and from each other.
const ENVIRONMENTS: &[(&str, &str)] = &[
    ("C", "UTC"),
    ("en_US.UTF-8", "Asia/Kathmandu"),
    ("tr_TR.UTF-8", "Pacific/Chatham"),
];

/// A repository whose state is worth reporting on: documentation citing a file
/// that was never written, and an export nothing calls.
fn untidy_repository() -> Repo {
    let repo = Repo::init();
    repo.write(
        "README.md",
        "# demo\n\nThe parser lives in `src/parser.ts`, and how it decides is in `docs/parser.md`.\n",
    );
    repo.write(
        "src/parser.ts",
        "export function parseInput(text: string): string[] {\n  return text.split(\",\");\n}\n\nexport function formatRecord(fields: string[]): string {\n  return fields.join(\",\");\n}\n",
    );
    repo
}

fn results(stdout: &str) -> Vec<Value> {
    let log: Value = serde_json::from_str(stdout)
        .unwrap_or_else(|error| panic!("stdout should be a SARIF log: {error}\n{stdout}"));
    log["runs"][0]["results"]
        .as_array()
        .expect("a run carries a results array")
        .clone()
}

#[test]
fn the_same_tree_answers_the_same_way_untracked_staged_and_committed() {
    let repo = untidy_repository();

    let untracked = repo.weeder(&["scan", "--format", "sarif"]);
    repo.stage_all();
    let staged = repo.weeder(&["scan", "--format", "sarif"]);
    repo.commit("the parser lands");
    let committed = repo.weeder(&["scan", "--format", "sarif"]);

    let found = results(&untracked.stdout);
    assert!(
        found.len() >= 2,
        "the fixture is untidy in two ways and the scan should say so: {found:#?}"
    );
    assert_eq!(
        found,
        results(&staged.stdout),
        "staging a file changes nothing about the state the repository is in"
    );
    assert_eq!(
        found,
        results(&committed.stdout),
        "committing a file changes nothing about the state the repository is in"
    );
    for run in [&untracked, &staged, &committed] {
        assert_eq!(run.code, 0, "a scan never blocks: {}", run.stderr);
    }
}

#[test]
fn a_change_the_working_tree_makes_is_judged_and_the_committed_state_is_not() {
    let repo = untidy_repository();
    repo.commit("the parser lands");
    let before = repo.weeder(&["scan", "--format", "sarif"]);

    // The file the docs were missing is written and never staged. A diff would
    // see an added file; a scan sees a repository that is no longer wrong.
    repo.write(
        "docs/parser.md",
        "# How the parser decides\n\nAt a comma.\n",
    );
    let after = repo.weeder(&["scan", "--format", "sarif"]);

    let citations = |stdout: &str| {
        results(stdout)
            .into_iter()
            .filter(|result| result["ruleId"] == "R1")
            .count()
    };
    assert_eq!(
        citations(&before.stdout),
        1,
        "the doc cited a file nobody wrote"
    );
    assert_eq!(
        citations(&after.stdout),
        0,
        "an unstaged, uncommitted file is part of the tree, and the citation now resolves"
    );
}

#[test]
fn a_scan_that_finds_warnings_leaves_with_zero() {
    let repo = untidy_repository();
    let run = repo.weeder(&["scan", "--format", "sarif"]);
    assert_eq!(run.code, 0);
    assert_eq!(run.stderr, "");

    let found = results(&run.stdout);
    assert!(!found.is_empty(), "the scan found nothing to report");
    for result in &found {
        assert_eq!(
            result["level"], "warning",
            "every scan finding is a warning: {result:#?}"
        );
    }
    let invocation =
        &serde_json::from_str::<Value>(&run.stdout).expect("a log")["runs"][0]["invocations"][0];
    assert_eq!(invocation["executionSuccessful"], true);
    assert_eq!(invocation["exitCode"], 0);
}

#[test]
fn a_scan_outside_a_repository_leaves_with_three_and_says_why() {
    let anywhere = TempDir::new().expect("a directory to run in");
    let run = weeder_in(anywhere.path(), &["scan", "--format", "sarif"]);

    assert_eq!(run.code, 3, "a run that judged nothing fails closed");
    assert!(
        run.stderr.contains("not inside a git repository"),
        "the message should say what stopped it: {}",
        run.stderr
    );
    let log: Value = serde_json::from_str(&run.stdout).expect("stdout should still be a log");
    let invocation = &log["runs"][0]["invocations"][0];
    assert_eq!(invocation["executionSuccessful"], false);
    assert_eq!(invocation["exitCode"], 3);
    assert!(
        invocation["toolExecutionNotifications"][0]["message"]["text"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
    );
}

#[test]
fn a_check_rule_asked_of_the_scan_face_is_a_run_that_never_happened() {
    let repo = untidy_repository();
    for (asked, expected) in [("T1", "check rule"), ("R9", "knows no rule")] {
        let run = repo.weeder(&["scan", "--rules", asked, "--format", "sarif"]);
        assert_eq!(
            run.code, 3,
            "weeder should refuse `--rules {asked}` rather than quietly answer a smaller question"
        );
        assert!(
            run.stderr.contains(expected),
            "the message should say why: {}",
            run.stderr
        );
    }
}

#[test]
fn rules_narrows_the_run_and_a_comma_separates_them() {
    let repo = untidy_repository();
    let only_r1 = repo.weeder(&["scan", "--rules", "R1", "--format", "sarif"]);
    let both = repo.weeder(&["scan", "--rules", "R1,R2", "--format", "sarif"]);

    let rules = |stdout: &str| {
        let mut found: Vec<String> = results(stdout)
            .into_iter()
            .map(|result| result["ruleId"].as_str().unwrap_or_default().to_string())
            .collect();
        found.sort();
        found.dedup();
        found
    };
    assert_eq!(rules(&only_r1.stdout), vec!["R1".to_string()]);
    assert_eq!(
        rules(&both.stdout),
        vec!["R1".to_string(), "R2".to_string()]
    );
}

/// The terminal half of this is a pty, made with `openpty`, which Windows has
/// not; its own pseudo-console is another API no crate this suite binds. The
/// table itself is held to its shape on every platform by the test below.
#[cfg(unix)]
#[test]
fn a_pipe_gets_sarif_a_terminal_gets_a_table_and_format_overrides_both() {
    let repo = untidy_repository();

    let piped = repo.weeder(&["scan"]);
    assert!(
        piped.stdout.trim_start().starts_with('{'),
        "a pipe gets the log: {}",
        piped.stdout
    );

    let terminal = repo.weeder_on_a_terminal(&["scan"]);
    assert!(
        terminal.stdout.contains("warning"),
        "a terminal gets the table: {}",
        terminal.stdout
    );

    let forced = repo.weeder(&["scan", "--format", "table"]);
    assert_eq!(
        forced.stdout, terminal.stdout,
        "--format table is the same table the terminal got"
    );
}

#[test]
fn the_table_reads_like_the_one_check_writes() {
    let repo = untidy_repository();
    let run = repo.weeder(&["scan", "--format", "table"]);
    assert_eq!(run.code, 0);

    let lines: Vec<&str> = run.stdout.lines().collect();
    let (counts, findings) = lines.split_last().expect("the table ends with its counts");
    assert!(!findings.is_empty(), "the table should carry the findings");
    for line in findings {
        let cells: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(cells[0], "warning", "the level comes first: {line}");
        assert!(cells[1].starts_with('R'), "then the rule id: {line}");
        assert!(cells[2].contains(':'), "then path and line: {line}");
    }
    assert_eq!(
        counts,
        &format!("0 errors, {} warnings, 0 notes", findings.len()),
        "the last line counts each level"
    );
}

#[test]
fn the_log_declares_the_scan_rules_and_no_others() {
    let repo = untidy_repository();
    let run = repo.weeder(&["scan", "--format", "sarif"]);
    let log: Value = serde_json::from_str(&run.stdout).expect("a log");
    let declared: Vec<String> = log["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("the tool component lists its rules")
        .iter()
        .map(|rule| rule["id"].as_str().unwrap_or_default().to_string())
        .collect();
    let expected: Vec<String> = catalogue::rules()
        .iter()
        .filter(|rule| rule.face == Face::Scan)
        .map(|rule| rule.id.to_string())
        .collect();
    assert_eq!(
        declared, expected,
        "a scan log declares what a scan looks for, and a check rule was never among them"
    );
}

#[test]
fn a_scan_writes_the_same_bytes_in_every_locale_and_time_zone() {
    let repo = untidy_repository();
    repo.commit("the parser lands");

    for arguments in [
        vec!["scan", "--format", "sarif"],
        vec!["scan", "--format", "table"],
        vec!["scan", "--rules", "r1", "--format", "sarif"],
    ] {
        let replays: Vec<Run> = ENVIRONMENTS
            .iter()
            .map(|(locale, zone)| in_locale(&repo, &arguments, locale, zone))
            .collect();
        let first = &replays[0];
        assert!(
            !first.stdout.is_empty(),
            "`weeder {}` wrote nothing, so the replays compare nothing",
            arguments.join(" ")
        );
        for (replay, (locale, zone)) in replays.iter().zip(ENVIRONMENTS) {
            assert_eq!(
                replay.stdout,
                first.stdout,
                "`weeder {}` wrote different bytes under LANG={locale} TZ={zone}",
                arguments.join(" ")
            );
            assert_eq!(
                replay.code,
                first.code,
                "`weeder {}` left with a different code under LANG={locale} TZ={zone}",
                arguments.join(" ")
            );
        }
    }
}

/// A scan may report a warning and it may report nothing. The exit code this
/// face hands back is 0 either way, and that is only honest while no rule on
/// the face carries a level that would mean otherwise.
#[test]
fn no_rule_on_this_face_carries_a_blocking_level() {
    for rule in catalogue::rules()
        .iter()
        .filter(|rule| rule.face == Face::Scan)
    {
        assert_eq!(
            rule.default_level,
            Level::Warn,
            "{} would block, and a scan leaves with 0 whatever it finds",
            rule.id
        );
    }
}

#[test]
fn every_result_carries_its_path_for_a_consumer_that_aggregates_by_file() {
    let repo = untidy_repository();
    let run = repo.weeder(&["scan", "--format", "sarif"]);
    let found = results(&run.stdout);
    assert!(!found.is_empty());
    for result in &found {
        let located = &result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"];
        assert_eq!(
            &result["properties"]["path"], located,
            "properties.path repeats the location's own uri: {result:#?}"
        );
    }
}

/// The built binary in this repository, in a locale and a time zone of the
/// caller's choosing. Everything else about the runs is the same.
fn in_locale(repo: &Repo, arguments: &[&str], locale: &str, zone: &str) -> Run {
    let mut command = repo.weeder_command(arguments);
    let output = with_locale(&mut command, locale, zone)
        .output()
        .expect("the weeder binary should run");
    Run {
        code: output
            .status
            .code()
            .expect("weeder should leave with a code"),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

fn with_locale<'a>(command: &'a mut Command, locale: &str, zone: &str) -> &'a mut Command {
    command
        .env("LANG", locale)
        .env("LC_ALL", locale)
        .env("LANGUAGE", locale)
        .env("TZ", zone)
}
