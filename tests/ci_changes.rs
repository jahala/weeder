//! A pull request that changes only prose skips the latency job (#35).
//!
//! The latency job runs the whole suite a second time on the release profile,
//! so that the budget in `tests/speed.rs` is measured on the binary people run.
//! Prose changes neither that binary nor the fixtures the budget times, so a
//! pull request made of prose alone has nothing for the second run to find. The
//! check job still runs on it: the self-check judges every file in the tree,
//! prose included, and several tests read the docs.
//!
//! `scripts/ci-changes.sh <base> <head>` asks git what the range changed and
//! decides, and the workflow asks it. Every test here builds a real repository
//! and a real range, so what the script reads is what git writes, renames
//! included. Anything it cannot place as prose is code, and so is a range git
//! cannot answer for.

mod common;

use std::path::Path;

use common::Repo;

/// A repository whose first commit holds `base`, and whose second makes the
/// change `change` writes. The answer is the script's, for the range between.
fn answer(base: &[(&str, &str)], change: impl Fn(&Repo)) -> String {
    let repo = Repo::init();
    for (path, text) in base {
        repo.write(path, text);
    }
    repo.commit("the base");
    let from = repo.head();
    change(&repo);
    repo.commit("the change");
    run(&repo, &from, "HEAD")
}

/// The script's one line, run inside the repository on a range.
fn run(repo: &Repo, base: &str, head: &str) -> String {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/ci-changes.sh");
    let output = common::bash(&script)
        .arg(base)
        .arg(head)
        .current_dir(repo.root())
        .output()
        .expect("bash should run the script");
    assert!(
        output.status.success(),
        "the script exits 0 whatever it decides: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

const CODE: &[(&str, &str)] = &[
    ("src/lib.rs", "pub fn one() -> u32 { 1 }\n"),
    ("README.md", "# notes\n"),
];

#[test]
fn a_change_made_only_of_prose_is_not_code() {
    let said = answer(CODE, |repo| {
        for path in [
            "README.md",
            "docs/rules.md",
            "docs/tend/release.tend.html",
            "docs/calibration/corpus.toml",
            ".brand/products/weeder/voice.md",
            "LICENSE",
        ] {
            repo.write(path, "words, rewritten\n");
        }
    });
    assert_eq!(said, "code=false\n");
}

#[test]
fn one_path_outside_prose_makes_the_change_code() {
    for path in [
        "src/core/mod.rs",
        "tests/speed.rs",
        "fixtures/adversarial/S2/sh/fire/after/scripts/sync.sh",
        ".github/workflows/ci.yml",
        "Cargo.lock",
        "garden.json",
        "scripts/check/run.sh",
    ] {
        let said = answer(CODE, |repo| {
            repo.write("README.md", "# notes, rewritten\n");
            repo.write(path, "changed\n");
        });
        assert_eq!(
            said, "code=true\n",
            "{path} changes what the release suite runs"
        );
    }
}

/// The suite reads markdown of its own: a fixture's `AGENTS.md` is the input a
/// rule is proven on, and a test's prose is part of the test. Below `fixtures/`
/// and `tests/`, markdown is code (#159).
#[test]
fn markdown_the_suite_reads_is_code() {
    for path in [
        "fixtures/adversarial/C1/agents/fire/after/AGENTS.md",
        "tests/fixtures/notes.md",
    ] {
        let said = answer(CODE, |repo| repo.write(path, "the law, rewritten\n"));
        assert_eq!(said, "code=true\n", "{path} is read by the suite");
    }
}

/// A move from code to prose deletes code. git lists a rename by its new path
/// alone unless it is told not to look for renames, and the new path here is
/// prose (#159).
#[test]
fn a_rename_out_of_code_is_code() {
    let said = answer(
        &[
            (
                "src/notes.rs",
                "pub fn notes() -> &'static str { \"notes\" }\n",
            ),
            ("README.md", "# notes\n"),
        ],
        |repo| {
            // The same bytes at a new path: git reads it as a rename.
            repo.write(
                "docs/notes.md",
                "pub fn notes() -> &'static str { \"notes\" }\n",
            );
            repo.remove("src/notes.rs");
        },
    );
    assert_eq!(said, "code=true\n");
}

#[test]
fn a_range_git_cannot_answer_for_is_code() {
    let repo = Repo::init();
    repo.write("README.md", "# notes\n");
    repo.commit("the base");
    assert_eq!(run(&repo, "no-such-ref", "HEAD"), "code=true\n");
}

/// A path is read whole even with a newline in it. Read line by line, this one
/// would be `docs/one` and `two.rs`, and the second would look like code.
/// Windows keeps no newline in a file name, so the case is unix's alone.
#[cfg(unix)]
#[test]
fn a_path_is_read_whole_even_with_a_newline_in_it() {
    let said = answer(CODE, |repo| repo.write("docs/one\ntwo.rs", "words\n"));
    assert_eq!(said, "code=false\n");
}

/// A job's block in the workflow: the lines indented under its header, up to
/// the next key written at a job's depth or shallower. Comments are skipped,
/// since a comment above the next job sits at a job's depth too.
fn job(workflow: &str, name: &str) -> String {
    let header = format!("  {name}:");
    let mut lines = workflow.lines().skip_while(|line| *line != header);
    assert!(lines.next().is_some(), "ci.yml has a `{name}` job");
    lines
        .take_while(|line| {
            let body = line.trim_start();
            let depth = line.len() - body.len();
            body.is_empty() || body.starts_with('#') || depth > 2
        })
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_workflow_skips_only_the_latency_job_and_only_on_prose() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let workflow = std::fs::read_to_string(&path).expect("the CI workflow should be readable");

    let changes = job(&workflow, "changes");
    assert!(
        changes.contains("bash scripts/ci-changes.sh \"$BASE\" \"$HEAD\""),
        "the changes job hands the script the range, and the script asks git:\n{changes}"
    );

    let latency = job(&workflow, "latency");
    assert!(
        latency.contains("needs: changes"),
        "latency waits for the answer:\n{latency}"
    );
    // A changes job that failed answers nothing, and a job whose `needs` failed
    // is skipped unless its condition says `always()`. A skipped job passes a
    // required check, so latency runs unless the answer is plainly prose.
    assert!(
        latency.contains(
            "if: always() && (github.event_name != 'pull_request' || needs.changes.outputs.code != 'false')"
        ),
        "latency skips a pull request of prose, and nothing else, a failed answer included:\n{latency}"
    );

    let check = job(&workflow, "check");
    assert!(
        !check.contains("if:") && !check.contains("needs:"),
        "check runs on every change, prose included:\n{check}"
    );
}
