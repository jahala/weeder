//! A pull request that changes only prose skips the latency job (#35).
//!
//! The latency job runs the whole suite a second time on the release profile,
//! so that the budget in `tests/speed.rs` is measured on the binary people run.
//! Prose changes neither that binary nor the fixtures the budget times, so a
//! pull request made of prose alone has nothing for the second run to find. The
//! check job still runs on it: the self-check judges every file in the tree,
//! prose included, and several tests read the docs.
//!
//! `scripts/ci-changes.sh` decides, from the paths a pull request changed, and
//! the workflow asks it. Anything it cannot place as prose is code, and an
//! empty list is code too, because a list git failed to write says nothing.

mod common;

use std::io::Write;
use std::path::Path;
use std::process::Stdio;

/// What the script answers for these paths, written the way the workflow
/// hands them over: each path ended by a NUL, as `git diff -z` writes them.
fn answer(paths: &[&str]) -> String {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/ci-changes.sh");
    let mut child = common::bash(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bash should run the script");
    let mut input = Vec::new();
    for path in paths {
        input.extend_from_slice(path.as_bytes());
        input.push(0);
    }
    child
        .stdin
        .take()
        .expect("the script's stdin is piped")
        .write_all(&input)
        .expect("the paths reach the script");
    let output = child.wait_with_output().expect("the script finishes");
    assert!(
        output.status.success(),
        "the script exits 0 whatever it decides: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn a_change_made_only_of_prose_is_not_code() {
    let prose = [
        "README.md",
        "docs/rules.md",
        "docs/tend/release.tend.html",
        "docs/calibration/corpus.toml",
        ".brand/products/weeder/voice.md",
        "LICENSE",
    ];
    assert_eq!(answer(&prose), "code=false\n");
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
        assert_eq!(
            answer(&["README.md", path]),
            "code=true\n",
            "{path} changes what the release suite runs"
        );
    }
}

#[test]
fn an_empty_list_is_code_because_it_says_nothing() {
    assert_eq!(answer(&[]), "code=true\n");
}

#[test]
fn a_path_is_read_whole_even_with_a_newline_in_it() {
    // Read line by line, this one path would be `docs/one` and `two.rs`, and
    // the second would look like code. It is one file under `docs/`.
    assert_eq!(answer(&["docs/one\ntwo.rs"]), "code=false\n");
    // And the other way: a path that only looks like prose on its first line.
    assert_eq!(answer(&["README.md\n/../src/lib.rs"]), "code=true\n");
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
        changes.contains("scripts/ci-changes.sh"),
        "the changes job asks the script:\n{changes}"
    );
    assert!(
        changes.contains("git diff -z --name-only"),
        "the changes job hands the script whole paths:\n{changes}"
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
