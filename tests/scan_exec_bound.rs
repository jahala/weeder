//! What a scan may execute is a closed list, and the list is what runs.
//!
//! R1 resolves a cited command against the command's own help, so a scan starts
//! processes. A document is prose, and prose holds whatever somebody typed into
//! it: another repository's tool, a line with a semicolon in it, the delete
//! somebody warns against. None of that is weed's to run. The bound is that
//! weed runs only a program `[docs] commands` names, only with `--help`, walking
//! only the subcommands that help itself printed, and never through a shell.
//!
//! Nothing here is mocked: the fixture is a real repository, the binary is the
//! built one, and PATH is a directory this test owns whose every program writes
//! down the arguments it was handed. What weed ran is then read off those
//! recordings rather than promised. The document in the fixture cites a
//! subcommand the tool no longer offers, so a scan that quietly skipped the
//! document would fail here too.

mod common;

use std::path::Path;

use common::{fixture, install_script, shell_word, which, Finding};
use tempfile::TempDir;

/// The one command the fixture's `[docs] commands` names.
const LISTED: &str = "listed-tool";

/// What weed ran, in order, and all it may run beside git: the listed command
/// asked for its own help, and the one subcommand that help printed.
const ALLOWED: &[&str] = &["listed-tool --help", "listed-tool build --help"];

/// Programs the fixture's document names or implies, none of which weed may
/// reach for: the tool the config does not list, the shells a command line
/// would go through, the delete and the touch the prose types out, and the
/// fetcher that belongs to a refresh nobody asked for.
const FORBIDDEN: &[&str] = &[
    "unlisted-tool",
    "sh",
    "bash",
    "zsh",
    "env",
    "rm",
    "touch",
    "curl",
];

/// What a shell would leave behind if weed handed it the line the document
/// carries. The file never appears, because no shell is ever asked.
const SHELL_LEAVINGS: &str = "pwned";

#[test]
fn a_scan_runs_the_listed_command_and_nothing_else() {
    let tools = TempDir::new().expect("a directory for the commands this test owns");
    let log = tools.path().join("argv.log");
    install_script(tools.path(), "git", &recording_git(&log));
    install_script(tools.path(), LISTED, &listed_tool(&log));
    for program in FORBIDDEN {
        install_script(tools.path(), program, &recorder(program, &log));
    }

    let repo = fixture("R1", "commands", "fire-bound");
    let run = repo.weed_with(
        &["scan", "--format", "sarif"],
        &[("PATH", &tools.path().display().to_string())],
    );
    assert_eq!(
        run.code, 0,
        "a scan never blocks, and this one left with {}: {}",
        run.code, run.stderr
    );

    let ran = ran(&log);
    assert!(
        ran.iter().any(|line| line.starts_with("git ")),
        "weed asks git what the tree holds before any rule runs, so a log without git is a log nothing was written to: {ran:#?}"
    );
    let commands: Vec<&String> = ran
        .iter()
        .filter(|line| !line.starts_with("git "))
        .collect();
    assert_eq!(
        commands, ALLOWED,
        "the scan ran something outside the closed list, or ran the listed command with an argument it was not given by its own help"
    );

    assert!(
        !repo.root().join(SHELL_LEAVINGS).exists() && !tools.path().join(SHELL_LEAVINGS).exists(),
        "the document's `{LISTED}; touch {SHELL_LEAVINGS}` reached a shell: weed hands a program and an argument array, never a command line"
    );
}

#[test]
fn the_document_is_still_judged_against_the_help_that_was_read() {
    let tools = TempDir::new().expect("a directory for the commands this test owns");
    let log = tools.path().join("argv.log");
    install_script(tools.path(), "git", &recording_git(&log));
    install_script(tools.path(), LISTED, &listed_tool(&log));
    for program in FORBIDDEN {
        install_script(tools.path(), program, &recorder(program, &log));
    }

    let repo = fixture("R1", "commands", "fire-bound");
    let run = repo.weed_with(
        &["scan", "--rules", "R1", "--format", "sarif"],
        &[("PATH", &tools.path().display().to_string())],
    );
    assert_eq!(run.code, 0, "a scan never blocks: {}", run.stderr);

    let found = run.findings();
    assert!(
        found.iter().any(|finding| message(finding).contains("deploy")),
        "the help says the tool offers `build` alone, so the cited `deploy` is the finding this fixture exists to produce: {found:#?}"
    );
    assert!(
        !found
            .iter()
            .any(|finding| message(finding).contains("unlisted-tool")),
        "a command `[docs] commands` does not name has no authority weed can ask, so weed says nothing about it: {found:#?}"
    );
    assert!(
        !found
            .iter()
            .any(|finding| message(finding).contains("build --verbose")),
        "`build` and `--verbose` are what the two helps printed, so the citation that uses them resolves: {found:#?}"
    );
}

fn message(finding: &Finding) -> &str {
    &finding.message
}

/// Every command a test-owned program recorded, in the order they ran.
fn ran(log: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(log).unwrap_or_default();
    text.lines().map(str::to_string).collect()
}

/// The line a test-owned program writes about itself before it does anything
/// else: its own name and the arguments it was handed, one invocation to a
/// line. This is the whole proof, so it is written by the programs themselves
/// rather than by anything weed could route around.
fn record(program: &str, log: &Path) -> String {
    format!(
        "{{ printf '%s' {name}; for word in \"$@\"; do printf ' %s' \"$word\"; done; printf '\\n'; }} >> {log}",
        name = shell_word(program),
        log = shell_word(&log.display().to_string()),
    )
}

/// A program that records being called and does nothing at all. Every program
/// weed must not reach for is one of these, so reaching for one is a line in
/// the log rather than a thing that happened.
fn recorder(program: &str, log: &Path) -> String {
    format!("#!/bin/sh\n{}\nexit 0\n", record(program, log))
}

/// git, recorded and then run for real. weed asks git what the tree holds
/// before any rule runs, so a PATH without git is a scan that never starts.
fn recording_git(log: &Path) -> String {
    let found = which("git").expect("git should be on PATH");
    format!(
        "#!/bin/sh\n{}\nexec {} \"$@\"\n",
        record("git", log),
        shell_word(&found.display().to_string()),
    )
}

/// The one command `[docs] commands` names: a CLI with a single subcommand and
/// two flags, which prints its help and refuses anything else the way a real
/// one does. What the document cites beyond that, `deploy` and `--frobnicate`,
/// this tool has never heard of. It reaches `cat` by its path, because the PATH
/// this test hands weed holds nothing but the programs the test wrote.
fn listed_tool(log: &Path) -> String {
    format!(
        r#"#!/bin/sh
{record}
case "$1" in
  --help)
    /bin/cat <<'HELP'
Usage: {LISTED} <COMMAND>

Commands:
  build  Build the thing

Options:
  -v, --verbose  Say more about what happened
  -h, --help     Print help
HELP
    ;;
  build)
    /bin/cat <<'HELP'
Usage: {LISTED} build [OPTIONS]

Options:
      --verbose  Say more about what happened
  -h, --help     Print help
HELP
    ;;
  *)
    echo "{LISTED}: no command called $1" >&2
    exit 2
    ;;
esac
"#,
        record = record(LISTED, log),
    )
}
