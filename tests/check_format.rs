//! What `weeder check` writes, and to whom.
//!
//! A pipe gets SARIF, because whatever is reading it is a program. A terminal
//! gets the table, because whatever is reading it is a person. `--format` says
//! so explicitly and wins over both. The terminal here is a real pseudo-terminal
//! on the binary's stdout, so weeder answers the question rather than being told
//! the answer.

mod common;

use common::{conflicted_parser, Repo, Run};

fn conflicted() -> Repo {
    let repo = Repo::init();
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();
    repo
}

#[test]
fn a_pipe_gets_sarif() {
    let run = conflicted().weeder(&["check"]);

    assert_sarif(&run);
    assert_eq!(run.code, 2);
}

#[test]
fn a_terminal_gets_a_table() {
    let run = conflicted().weeder_on_a_terminal(&["check"]);

    assert_table(&run);
    assert_eq!(run.code, 2);
}

#[test]
fn format_table_overrides_the_pipe() {
    let run = conflicted().weeder(&["check", "--format", "table"]);

    assert_table(&run);
    assert_eq!(run.code, 2);
}

#[test]
fn format_sarif_overrides_the_terminal() {
    let run = conflicted().weeder_on_a_terminal(&["check", "--format", "sarif"]);

    assert_sarif(&run);
    assert_eq!(run.code, 2);
}

fn assert_sarif(run: &Run) {
    let log = run.log();
    assert_eq!(log["version"], "2.1.0", "the log declares its version");
    assert!(
        log["$schema"]
            .as_str()
            .is_some_and(|uri| uri.contains("sarif")),
        "the log names the schema a consumer validates against"
    );
    assert!(
        !run.findings().is_empty(),
        "the findings travel in the log, not beside it"
    );
}

fn assert_table(run: &Run) {
    assert!(
        !run.stdout.trim_start().starts_with('{'),
        "a table is not a log:\n{}",
        run.stdout
    );

    let lines = run.stdout_lines();
    let count = lines.last().expect("the table ends with a count");
    assert!(
        count.contains("error") && count.contains("warning") && count.contains("note"),
        "the last line counts each level, and it read `{count}`"
    );

    let first = lines.first().expect("a finding has a line of its own");
    let cells: Vec<&str> = first.split_whitespace().collect();
    assert_eq!(cells[0], "error", "the level comes first");
    assert_eq!(cells[1], "G1", "then the rule");
    assert!(
        cells[2].starts_with("src/parser.ts:"),
        "then where it is, and it read `{}`",
        cells[2]
    );
}
