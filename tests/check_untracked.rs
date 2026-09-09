//! The files an agent wrote and never staged.
//!
//! `git diff HEAD` never lists a file that was never added, so weeder used to
//! look straight past most of what an agent produces: an audit of a 151-file
//! tree found 131 of them untracked, plain `check` seeing three errors where the
//! staged tree had twenty. The end of a turn is the moment weeder exists for, so
//! that is where the untracked files come in, as what they are: added files,
//! with no side before them and their after side read off the disk.
//!
//! Only the two modes that judge the working tree take them by default. The
//! index is what a commit carries, so `--staged` has nothing to say about a file
//! the index does not hold; a `--base` range and a pre-push range are histories,
//! and a working tree is not part of one. `--untracked` says so explicitly
//! wherever the default is not what a caller wants.

mod common;

use common::Repo;
use serde_json::json;

/// The size G2 draws the line at.
const MEBIBYTE: usize = 1024 * 1024;

/// A file of readable text, larger than that line.
fn big_text() -> String {
    let row = "the quick brown fox jumps over the lazy dog\n";
    row.repeat(MEBIBYTE / row.len() + 2)
}

/// A test file with a skip marker in it: what an agent leaves behind when the
/// case it could not make pass had to stop failing.
const SKIPPED: &str = "import { format } from \"./format\";\n\n\
                       describe(\"format\", () => {\n  \
                       it(\"pads to the width\", () => {\n    \
                       expect(format(\"a\", 3)).toBe(\"a  \");\n  });\n\n  \
                       it.skip(\"truncates past the width\", () => {\n    \
                       expect(format(\"abcd\", 3)).toBe(\"abc\");\n  });\n});\n";

/// The production file the suite is about, committed so the repository's only
/// change is the untracked test file beside it.
const FORMAT: &str = "export function format(text: string, width: number): string {\n  \
                      return text.padEnd(width);\n}\n";

/// A repository whose one change is an untracked test file carrying a marker.
fn only_an_untracked_test() -> Repo {
    let repo = Repo::init();
    repo.write("src/format.ts", FORMAT);
    repo.commit("the formatter");
    repo.write("src/format.test.ts", SKIPPED);
    repo
}

#[test]
fn an_untracked_file_is_judged_as_an_added_file() {
    let repo = only_an_untracked_test();

    let run = repo.weeder(&["check"]);
    let findings = run.findings();
    assert!(
        findings.iter().any(|finding| finding.rule == "T3"
            && finding.path == "src/format.test.ts"
            && finding.level == "error"),
        "the marker in the file nobody staged is the finding: {:?}",
        findings
    );
    assert_eq!(run.code, 2, "and it stops the turn");
}

#[test]
fn the_index_alone_still_means_the_index_alone() {
    let repo = only_an_untracked_test();

    let run = repo.weeder(&["check", "--staged"]);
    assert!(
        run.findings().is_empty(),
        "a commit carries the index, and the index does not carry this file: {}",
        run.stdout
    );
    assert_eq!(run.code, 0);
}

#[test]
fn a_base_range_is_a_history_and_leaves_the_untracked_file_out() {
    let repo = only_an_untracked_test();
    let base = repo.head();

    let run = repo.weeder(&["check", "--base", &base]);
    assert!(
        run.findings().is_empty(),
        "what a branch changed is what its commits changed: {}",
        run.stdout
    );
    assert_eq!(run.code, 0);
}

#[test]
fn untracked_exclude_puts_the_old_view_back_and_include_asks_for_it_anywhere() {
    let repo = only_an_untracked_test();
    let base = repo.head();

    let excluded = repo.weeder(&["check", "--untracked", "exclude"]);
    assert!(
        excluded.findings().is_empty(),
        "the file is left out where the caller says so: {}",
        excluded.stdout
    );
    assert_eq!(excluded.code, 0);

    let included = repo.weeder(&["check", "--base", &base, "--untracked", "include"]);
    assert!(
        included
            .findings()
            .iter()
            .any(|finding| finding.rule == "T3"),
        "and asked for against a ref, the working tree's own files come with it: {}",
        included.stdout
    );
    assert_eq!(included.code, 2);
}

#[test]
fn the_index_cannot_be_asked_for_files_it_does_not_hold() {
    let repo = only_an_untracked_test();

    let refused = repo.weeder(&["check", "--staged", "--untracked", "include"]);
    assert_eq!(
        refused.code, 3,
        "a run that cannot do what it was asked never happened: {}",
        refused.stdout
    );
    assert_eq!(
        refused.stderr_lines().len(),
        1,
        "one line, naming the cause: {}",
        refused.stderr
    );
    assert!(
        refused.stderr.contains("--staged") && refused.stderr.contains("--untracked"),
        "and naming both flags: {}",
        refused.stderr
    );
}

#[test]
fn the_same_tree_writes_the_same_bytes_twice() {
    let repo = only_an_untracked_test();
    repo.write("src/other.test.ts", SKIPPED);
    repo.write("docs/note.md", "The formatter pads.\n");

    let first = repo.weeder(&["check", "--format", "sarif"]);
    let second = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(
        first.stdout, second.stdout,
        "an untracked file arrives in the order the tree lists it, every run"
    );
    assert!(
        first.findings().len() >= 2,
        "and there is something to sort"
    );
}

#[test]
fn an_ignored_file_is_not_part_of_the_tree_at_all() {
    let repo = only_an_untracked_test();
    repo.write(".gitignore", "build/\n");
    repo.commit("the ignore rules");
    repo.write("build/generated.test.ts", SKIPPED);

    let run = repo.weeder(&["check"]);
    assert!(
        run.findings()
            .iter()
            .all(|finding| finding.path != "build/generated.test.ts"),
        "a file the ignore rules hide was never part of the change: {}",
        run.stdout
    );
}

#[test]
fn the_stop_event_sees_what_the_agent_left_behind() {
    let repo = only_an_untracked_test();

    let run = repo.weeder_reading(&["hook", "claude"], &stop_event(&repo));
    assert_eq!(
        run.code,
        2,
        "an agent cannot declare itself done over a file it never staged: {}",
        run.output()
    );
    assert!(
        run.output().contains("src/format.test.ts"),
        "and the refusal names the file: {}",
        run.output()
    );
}

#[test]
fn a_commit_the_harness_is_about_to_run_still_sees_the_index_alone() {
    let repo = only_an_untracked_test();

    let run = repo.weeder_reading(
        &["hook", "claude"],
        &tool_event(&repo, "git commit -m 'the formatter'"),
    );
    assert_eq!(
        run.code,
        0,
        "a commit carries the index, and the index is clean: {}",
        run.output()
    );
}

#[test]
fn the_pre_push_range_judges_the_commits_and_not_the_desk_they_were_written_on() {
    let repo = Repo::init();
    repo.write("src/format.ts", FORMAT);
    repo.commit("the formatter");
    let base = repo.head();
    repo.write("src/format.test.ts", FORMAT);
    repo.commit("a suite of sorts");
    repo.write("src/scratch.test.ts", SKIPPED);

    let run = repo.weeder(&["check", "--base", &base, "--untracked", "exclude"]);
    assert!(
        run.findings().is_empty(),
        "a range is a history, and an unstaged file is in none of it: {}",
        run.stdout
    );
}

#[test]
fn a_blob_nobody_staged_is_weighed_rather_than_read() {
    let repo = only_an_untracked_test();
    let filler = big_text();
    assert!(filler.len() > MEBIBYTE, "the blob is above the line");
    repo.write("assets/atlas.txt", &filler);

    let run = repo.weeder(&["check"]);
    assert!(
        run.findings()
            .iter()
            .any(|finding| finding.rule == "G2" && finding.path == "assets/atlas.txt"),
        "a megabyte arriving unstaged is a megabyte every clone carries: {}",
        run.stdout
    );
}

#[test]
fn a_file_nobody_staged_is_still_held_to_the_scope() {
    let repo = only_an_untracked_test();

    let run = repo.weeder(&["check", "--scope", "src/format.ts"]);
    assert!(
        run.findings()
            .iter()
            .any(|finding| finding.rule == "X2" && finding.path == "src/format.test.ts"),
        "the sentence a change is held to covers what was written, staged or not: {}",
        run.stdout
    );
}

#[test]
fn the_skill_names_the_flag_the_binary_prints() {
    let skill =
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("SKILL.md"))
            .expect("SKILL.md should be readable");
    assert!(
        skill.contains("--untracked"),
        "an agent reading the skill has to be told which files weeder judges"
    );
}

/// The event Claude Code writes when the agent has stopped talking.
fn stop_event(repo: &Repo) -> String {
    json!({
        "session_id": "a-session",
        "transcript_path": "/dev/null",
        "cwd": repo.root().display().to_string(),
        "hook_event_name": "Stop",
        "stop_hook_active": false,
    })
    .to_string()
}

/// The event Claude Code writes before it runs a command.
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
