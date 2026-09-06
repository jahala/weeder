//! C3, a workflow was changed.
//!
//! A workflow is a by-law rather than a constitution. It says how the checks
//! are run on a server, and it is edited legitimately every week, so weeder says
//! so and lets the change through: C3 warns wherever a file under
//! `.github/workflows/` is added, edited or taken away, and never blocks on its
//! own authority.
//!
//! The neighbour is the rest of `.github/`, which owns review and updates
//! rather than the run, and a directory somewhere else that happens to be
//! called workflows.
//!
//! A repository that publishes from a workflow can say so: naming the path
//! under `[guardrails] paths` in `weeder.toml` puts that one file back on the
//! constitution tier, and only that one.

mod common;

use common::{fixture, weeder_in, Finding};
use tempfile::TempDir;
use weeder::core::catalogue::{self, Face};
use weeder::core::finding::Level;

/// The fixture's languages folder. C3 reads paths, so one repository proves it.
const CASE: &str = "paths";

/// The page every SARIF result links back to.
const DOC: &str = "docs/rules.md";

/// The workflow files the fire fixture edits, adds and takes away.
const CHANGED: [&str; 3] = [
    ".github/workflows/ci.yml",
    ".github/workflows/nightly.yml",
    ".github/workflows/release.yml",
];

/// The file the promoted fixture holds at the constitution tier, and the one it
/// leaves as a by-law.
const PROMOTED: &str = ".github/workflows/release.yml";
const LEFT_ALONE: &str = ".github/workflows/ci.yml";

#[test]
fn c3_warns_on_a_workflow_added_edited_or_taken_away_and_blocks_nothing() {
    let repo = fixture("C3", CASE, "fire");
    let run = repo.weeder(&["check"]);

    assert_eq!(
        run.code, 0,
        "a by-law warns and lets the change through\n{}",
        run.stderr
    );
    let workflows = of_rule(&run.findings(), "C3");
    assert_eq!(
        paths(&workflows),
        CHANGED
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>(),
        "every workflow the change touched is named, however it touched it"
    );
    for finding in &workflows {
        assert_eq!(
            finding.level, "warning",
            "C3 warns: {} is for the person at the pull request",
            finding.path
        );
    }
}

#[test]
fn c3_stays_silent_on_the_rest_of_github_and_on_a_workflows_directory_elsewhere() {
    let repo = fixture("C3", CASE, "silent");

    let changed = repo.git(&["diff", "HEAD", "--name-only"]);
    for path in [
        ".github/CODEOWNERS",
        ".github/dependabot.yml",
        "deploy/workflows/ci.yml",
    ] {
        assert!(
            changed.lines().any(|line| line == path),
            "{path} must be in the diff, or the silence proves nothing"
        );
    }

    let run = repo.weeder(&["check"]);
    assert_eq!(
        of_rule(&run.findings(), "C3"),
        Vec::new(),
        "review, updates and a directory of the same name are not the run"
    );
    assert_eq!(run.code, 0, "nothing found, nothing blocked");
}

#[test]
fn a_repository_that_names_a_workflow_under_guardrails_paths_gets_it_at_block_level() {
    let repo = fixture("C3", CASE, "promoted");
    let run = repo.weeder(&["check"]);

    assert_eq!(
        run.code, 2,
        "the promoted workflow stops the commit\n{}",
        run.stderr
    );
    let workflows = of_rule(&run.findings(), "C3");
    let promoted = named(&workflows, PROMOTED);
    assert_eq!(
        promoted.level, "error",
        "the path `[guardrails] paths` names is this repository's constitution"
    );
    let by_law = named(&workflows, LEFT_ALONE);
    assert_eq!(
        by_law.level, "warning",
        "the workflow the repository did not name stays a by-law"
    );
}

#[test]
fn the_catalogue_the_page_and_the_binary_all_carry_c3_as_a_warn_rule() {
    let rule = catalogue::rule("C3").expect("the catalogue knows C3");
    assert_eq!(rule.face, Face::Check, "C3 judges a diff");
    assert_eq!(rule.default_level, Level::Warn, "C3 warns by default");

    let page = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(DOC))
        .expect("the rules page is readable");
    assert!(
        page.contains("<a id=\"C3\"></a>"),
        "{DOC} carries an anchor for C3, so its helpUri lands somewhere"
    );
    assert!(
        page.contains(rule.short_description) && page.contains(rule.full_description),
        "{DOC} says what C3 reports and what it reads to decide"
    );

    // Not a repository: the catalogue is weeder's own, not something it reads out
    // of a working tree.
    let anywhere = TempDir::new().expect("a directory to run in");
    let printed = weeder_in(anywhere.path(), &["rules"]);
    let row = printed
        .stdout_lines()
        .into_iter()
        .find(|line| line.starts_with("C3 "))
        .expect("`weeder rules` prints C3");
    let cells: Vec<&str> = row.split_whitespace().collect();
    assert_eq!(
        cells[1], "warn",
        "the printed level is the level weeder applies"
    );
    assert_eq!(cells[2], "check", "the printed face");
}

#[test]
fn a_c3_result_says_what_was_found_why_it_matters_and_what_to_do() {
    let repo = fixture("C3", CASE, "fire");
    let run = repo.weeder(&["check"]);
    for finding in of_rule(&run.findings(), "C3") {
        let sentences: Vec<&str> = finding
            .message
            .split(". ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        assert_eq!(
            sentences.len(),
            3,
            "what, why and next, and nothing else: {}",
            finding.message
        );
        assert!(
            sentences.iter().all(|part| part.len() > 10),
            "every part says something: {}",
            finding.message
        );
        assert!(
            finding.message.ends_with('.') && !finding.message.starts_with(char::is_uppercase),
            "sentence case, and it ends where it stops: {}",
            finding.message
        );
    }
}

/// The findings one rule reported, in the order weeder reported them.
fn of_rule(findings: &[Finding], rule: &str) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule == rule)
        .cloned()
        .collect()
}

/// The files a set of findings named, sorted and without repeats.
fn paths(findings: &[Finding]) -> Vec<String> {
    let mut paths: Vec<String> = findings
        .iter()
        .map(|finding| finding.path.clone())
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

fn named(findings: &[Finding], path: &str) -> Finding {
    findings
        .iter()
        .find(|finding| finding.path == path)
        .unwrap_or_else(|| panic!("a finding on {path}, among {findings:#?}"))
        .clone()
}
