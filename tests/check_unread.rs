//! The changed files weeder does not read (#79).
//!
//! weeder judges TypeScript, JavaScript, Python, Rust and Go. A change in any
//! other language reaches no rule, and a run over a repository whose whole
//! suite is bash used to end in `0 errors, 0 warnings, 0 notes`: a clean bill
//! over a diff nobody read. So `check` says, once per run and at note level,
//! which changed files it could not judge, by extension and count. The note
//! stops nothing; it keeps clean from passing for judged.

mod common;

use common::{Finding, Repo};

/// The id the note carries. It is a notice, not a rule.
const NOTICE: &str = "UNREAD";

/// A bash spec in the shape petals writes: an assertion is a `t` call, and the
/// grep pattern is the claim.
const SPEC: &str = "#!/usr/bin/env bash\n\
                    t \"prints the version\" 'weeder [0-9]' \"$(weeder --version)\"\n\
                    t \"prints the help\" 'Usage:' \"$(weeder --help)\"\n";

/// A production script.
const SCRIPT: &str = "#!/usr/bin/env bash\nset -euo pipefail\nweeder check \"$@\"\n";

fn repository() -> Repo {
    let repo = Repo::init();
    repo.write("tests/check.spec.sh", SPEC);
    repo.write("scripts/check.sh", SCRIPT);
    repo.write(
        "src/format.ts",
        "export const format = (text: string) => text;\n",
    );
    repo.commit("the state the change starts from");
    repo
}

fn notes(found: &[Finding]) -> Vec<&Finding> {
    found
        .iter()
        .filter(|finding| finding.rule == NOTICE)
        .collect()
}

#[test]
fn a_change_in_bash_alone_is_never_judged_clean_in_silence() {
    let repo = repository();
    // The assertion that went, and the debug line that came: what petals
    // planted, and what weeder could not see.
    repo.write(
        "tests/check.spec.sh",
        "#!/usr/bin/env bash\nt \"prints the version\" 'weeder [0-9]' \"$(weeder --version)\"\n",
    );
    repo.write(
        "scripts/check.sh",
        &format!("{SCRIPT}echo \"DEBUG weeder plant\"\n"),
    );
    repo.write("scripts/release.sh", SCRIPT);
    repo.stage_all();

    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(run.code, 0, "a note stops nothing: {}", run.output());
    let found = run.findings();
    let unread = notes(&found);
    assert_eq!(
        unread.len(),
        1,
        "one note for the run, however many files it covers: {found:#?}"
    );
    assert_eq!(unread[0].level, "note");
    assert!(
        unread[0].message.contains("3 .sh"),
        "the note counts the files by extension: {}",
        unread[0].message
    );
    assert_eq!(
        found.len(),
        1,
        "and it is the only thing weeder had to say: {found:#?}"
    );

    let table = repo.weeder(&["check", "--format", "table"]);
    assert!(
        table.stdout.contains(NOTICE) && table.stdout.ends_with("0 errors, 0 warnings, 1 note\n"),
        "the table says so too, and does not end clean:\n{}",
        table.stdout
    );
}

#[test]
fn each_unread_extension_is_counted_on_its_own_and_read_files_are_not_listed() {
    let repo = repository();
    repo.write("scripts/check.sh", &format!("{SCRIPT}exit 0\n"));
    repo.write("Makefile", "check:\n\tweeder check\n");
    repo.write("src/Report.java", "class Report {}\n");
    repo.write(
        "src/format.ts",
        "export const format = (text: string) => text.trim();\n",
    );
    repo.stage_all();

    let run = repo.weeder(&["check", "--format", "sarif"]);
    let found = run.findings();
    let unread = notes(&found);
    assert_eq!(unread.len(), 1, "one note: {found:#?}");
    let message = &unread[0].message;
    for counted in ["1 .java", "1 .sh", "1 Makefile"] {
        assert!(
            message.contains(counted),
            "the note counts {counted}: {message}"
        );
    }
    assert!(
        !message.contains(".ts"),
        "a file in a language weeder reads was judged, and is not listed: {message}"
    );
    assert!(
        message.find(".java") < message.find(".sh"),
        "the extensions come in one order, so two runs write one log: {message}"
    );
}

#[test]
fn a_change_weeder_reads_whole_draws_no_note() {
    let repo = repository();
    repo.write(
        "src/format.ts",
        "export const format = (text: string) => text.trim();\n",
    );
    repo.write("README.md", "# format\n");
    repo.write("config/settings.json", "{}\n");
    repo.stage_all();

    let run = repo.weeder(&["check", "--format", "sarif"]);
    let found = run.findings();
    assert!(
        notes(&found).is_empty(),
        "code weeder reads and files that are not code draw no note: {found:#?}"
    );
}

#[test]
fn every_mode_says_it_and_a_deleted_script_counts_too() {
    let repo = repository();
    repo.remove("scripts/check.sh");
    repo.stage_all();
    for arguments in [
        vec!["check", "--format", "sarif"],
        vec!["check", "--staged", "--format", "sarif"],
    ] {
        let run = repo.weeder(&arguments);
        let found = run.findings();
        let unread = notes(&found);
        assert_eq!(unread.len(), 1, "{arguments:?}: {found:#?}");
        assert!(
            unread[0].message.contains("1 .sh"),
            "{arguments:?}: a deleted script is a change weeder did not read: {}",
            unread[0].message
        );
    }

    repo.commit("the script goes");
    let run = repo.weeder(&["check", "--base", "HEAD~1", "--format", "sarif"]);
    let found = run.findings();
    assert_eq!(notes(&found).len(), 1, "a range says it too: {found:#?}");
}
