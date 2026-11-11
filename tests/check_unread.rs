//! The changed files weeder does not read (#79).
//!
//! weeder judges TypeScript, JavaScript, Python, Rust, Go and bash. A change in
//! any other language reaches no rule, and a run over a repository whose whole
//! suite was bash used to end in `0 errors, 0 warnings, 0 notes`: a clean bill
//! over a diff nobody read. So `check` says, once per run and at note level,
//! which changed files it could not judge, by extension and count. The note
//! stops nothing; it keeps clean from passing for judged. Bash is read now, and
//! the note stays for every language that is not.

mod common;

use common::{Finding, Repo};

/// The id the note carries. It is a notice, not a rule.
const NOTICE: &str = "UNREAD";

/// A Ruby spec. weeder has no rules for Ruby, so neither its claims nor its
/// leftovers reach one.
const SPEC: &str = "describe 'check' do\n\
                    \x20 it('prints the version') { expect(run('--version')).to match(/weeder/) }\n\
                    \x20 it('prints the help') { expect(run('--help')).to match(/Usage/) }\n\
                    end\n";

/// A production file in the same language.
const SCRIPT: &str = "def check(args)\n  system('weeder', 'check', *args)\nend\n";

fn repository() -> Repo {
    let repo = Repo::init();
    repo.write("spec/check_spec.rb", SPEC);
    repo.write("lib/check.rb", SCRIPT);
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
fn a_change_in_an_unread_language_alone_is_never_judged_clean_in_silence() {
    let repo = repository();
    // The assertion that went, and the debug line that came, in a language
    // weeder cannot see into.
    repo.write(
        "spec/check_spec.rb",
        "describe 'check' do\n  it('prints the version') { expect(run('--version')).to match(/weeder/) }\nend\n",
    );
    repo.write(
        "lib/check.rb",
        &format!("{SCRIPT}puts 'DEBUG weeder plant'\n"),
    );
    repo.write("lib/release.rb", SCRIPT);
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
        unread[0].message.contains("3 .rb"),
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
    repo.write("lib/check.rb", &format!("{SCRIPT}check([])\n"));
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
    for counted in ["1 .java", "1 .rb", "1 Makefile"] {
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
        message.find(".java") < message.find(".rb"),
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
    // Bash is a language weeder reads (#79): a script and a bats suite are
    // judged, and neither is counted as unread.
    repo.write(
        "scripts/check.sh",
        "#!/usr/bin/env bash\nweeder check \"$@\"\n",
    );
    repo.write(
        "test/check.bats",
        "@test \"runs\" {\n  run ./scripts/check.sh\n}\n",
    );
    repo.stage_all();

    let run = repo.weeder(&["check", "--format", "sarif"]);
    let found = run.findings();
    assert!(
        notes(&found).is_empty(),
        "code weeder reads, bash among it, and files that are not code draw no note: {found:#?}"
    );
}

#[test]
fn every_mode_says_it_and_a_deleted_script_counts_too() {
    let repo = repository();
    repo.remove("lib/check.rb");
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
            unread[0].message.contains("1 .rb"),
            "{arguments:?}: a deleted script is a change weeder did not read: {}",
            unread[0].message
        );
    }

    repo.commit("the script goes");
    let run = repo.weeder(&["check", "--base", "HEAD~1", "--format", "sarif"]);
    let found = run.findings();
    assert_eq!(notes(&found).len(), 1, "a range says it too: {found:#?}");
}

/// The change petals planted to find #79, in the shape petals writes: a spec
/// whose claims are calls to its own helper `t`, and a production script. Bash
/// is read now, so the claim that went blocks once the repository names its
/// helper, the debug line warns, and nothing is left unread.
#[test]
fn the_change_petals_planted_is_judged_now() {
    let claim = |what: &str, pattern: &str, flag: &str| {
        format!("t \"prints the {what}\" '{pattern}' \"$(weeder --{flag})\"\n")
    };
    let head = "#!/usr/bin/env bash\nsource tests/lib.sh\n\n";
    let script = "#!/usr/bin/env bash\nset -euo pipefail\nweeder check \"$@\"\n";
    let repo = Repo::init();
    repo.write("weeder.toml", "[tests]\nassertions = [\"t\"]\n");
    repo.write(
        "tests/check.spec.sh",
        &format!(
            "{head}{}{}",
            claim("version", "weeder [0-9]", "version"),
            claim("help", "Usage:", "help")
        ),
    );
    repo.write("scripts/check.sh", script);
    repo.commit("the state the change starts from");

    repo.write(
        "tests/check.spec.sh",
        &format!("{head}{}", claim("version", "weeder [0-9]", "version")),
    );
    repo.write(
        "scripts/check.sh",
        &format!("{script}echo \"DEBUG weeder plant\"\n"),
    );
    repo.stage_all();

    let run = repo.weeder(&["check", "--format", "sarif"]);
    let found = run.findings();
    let said: Vec<(&str, &str, &str)> = found
        .iter()
        .map(|finding| {
            (
                finding.rule.as_str(),
                finding.level.as_str(),
                finding.path.as_str(),
            )
        })
        .collect();
    assert_eq!(
        said,
        vec![
            ("S3", "warning", "scripts/check.sh"),
            ("T2", "error", "tests/check.spec.sh"),
        ],
        "the dropped claim and the debug line are both found, and nothing is unread: {found:#?}"
    );
    assert_eq!(run.code, 2, "the dropped claim stops the change");
}
