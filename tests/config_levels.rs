//! What a repository writes under `[rules]` is what a rule reports at.
//!
//! A detector may report one finding at a level of its own: D1 louder where a
//! manifest was never in scope, T2 quieter on a loop it cannot count, X1 a note
//! for a key a vendor published. The repository's own word still caps it. A rule
//! set to `warn` blocks nothing, whatever its detector knew, and a rule set to
//! `off` says nothing at all.
//!
//! This walks every check rule the binary's catalogue prints, and every fixture
//! each rule has in every language directory it keeps, except the silent ones,
//! which report nothing at any level. Each fixture is judged three times: under
//! its own law, under that law with the rule set to `warn`, and with the rule
//! set to `off`. Under `warn` the rule reports the same findings it reported
//! under its own law and none of them at error level; under `off` it reports
//! none. A `fire` fixture that reports its rule under its own law is required,
//! so the walk cannot pass by judging nothing.

mod common;

use std::path::{Path, PathBuf};

use tempfile::TempDir;

use common::{fixture, fixture_root, weeder_in, Finding, Repo, Run};

/// The file a fixture states its own law in.
const CONFIG: &str = "weeder.toml";

/// The check rules the binary's catalogue prints, in its order.
fn check_rules() -> Vec<String> {
    let run = weeder_in(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        &["rules", "--format", "json"],
    );
    assert_eq!(run.code, 0, "the catalogue should print: {}", run.stderr);
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(&run.stdout).expect("the catalogue should be json");
    let rules: Vec<String> = rows
        .iter()
        .filter(|row| row["face"] == "check")
        .map(|row| {
            row["id"]
                .as_str()
                .expect("the catalogue writes its ids as strings")
                .to_string()
        })
        .collect();
    assert!(!rules.is_empty(), "the catalogue should print check rules");
    rules
}

/// The directories below `path`, by name, sorted.
fn directories(path: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(path)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()))
        .map(|entry| entry.expect("a directory entry should read"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Every fixture a rule keeps that is not a silent one, as (language, case).
fn cells(rule: &str) -> Vec<(String, String)> {
    let root = fixture_root().join(rule);
    directories(&root)
        .into_iter()
        .flat_map(|lang| {
            directories(&root.join(&lang))
                .into_iter()
                .filter(|case| !case.starts_with("silent"))
                .map(move |case| (lang.clone(), case))
        })
        .collect()
}

/// The law the fixture's base commits, which is the law `check` reads when it
/// is not handed one.
fn own_law(repo: &Repo) -> String {
    let shown = repo.try_git(&["show", &format!("HEAD:{CONFIG}")]);
    if shown.code == 0 {
        shown.stdout
    } else {
        String::new()
    }
}

/// The fixture's own law with one rule set to `setting`, written where the run
/// can be pointed at it.
fn law_with(repo: &Repo, rule: &str, setting: &str, desk: &TempDir, name: &str) -> PathBuf {
    let own = own_law(repo);
    assert!(
        !own.contains("[rules]"),
        "{name} states its own rule levels, and cannot be handed another [rules] table"
    );
    let path = desk.path().join(format!("{name}-{setting}.toml"));
    std::fs::write(&path, format!("{own}\n[rules]\n{rule} = \"{setting}\"\n"))
        .expect("the law should be writable");
    path
}

fn check(repo: &Repo, config: Option<&Path>) -> Run {
    let mut arguments = vec!["check", "--format", "sarif"];
    let named;
    if let Some(config) = config {
        named = config.display().to_string();
        arguments.push("--config");
        arguments.push(&named);
    }
    repo.weeder(&arguments)
}

/// What a run reported from one rule, as place and message, sorted.
fn from_rule(run: &Run, rule: &str) -> Vec<Finding> {
    let mut found: Vec<Finding> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == rule)
        .collect();
    found.sort_by(|left, right| {
        (&left.path, left.line, &left.message).cmp(&(&right.path, right.line, &right.message))
    });
    found
}

fn places(found: &[Finding]) -> Vec<(String, Option<u64>, String)> {
    found
        .iter()
        .map(|finding| (finding.path.clone(), finding.line, finding.message.clone()))
        .collect()
}

#[test]
fn a_rule_set_to_warn_blocks_nothing_and_a_rule_set_to_off_says_nothing() {
    let desk = TempDir::new().expect("the laws need somewhere to sit");
    let mut wrong = Vec::new();

    for rule in check_rules() {
        let walked = cells(&rule);
        assert!(
            !walked.is_empty(),
            "{rule} keeps no fixture for this walk to judge"
        );
        for (lang, case) in walked {
            let name = format!("{rule}-{lang}-{case}");
            let cell = format!("fixtures/adversarial/{rule}/{lang}/{case}");

            let repo = fixture(&rule, &lang, &case);
            let own = check(&repo, None);
            assert_ne!(own.code, 3, "{cell} could not be judged\n{}", own.stderr);
            let reported = from_rule(&own, &rule);
            if case == "fire" && reported.is_empty() {
                wrong.push(format!("{cell} reported no {rule} under its own law"));
                continue;
            }

            let warned = check(&repo, Some(&law_with(&repo, &rule, "warn", &desk, &name)));
            assert_ne!(
                warned.code, 3,
                "{cell} could not be judged under warn\n{}",
                warned.stderr
            );
            let under_warn = from_rule(&warned, &rule);
            if places(&under_warn) != places(&reported) {
                wrong.push(format!(
                    "{cell} under {rule} = \"warn\" reported other findings than under its own law"
                ));
            }
            let loud: Vec<String> = under_warn
                .iter()
                .filter(|finding| finding.level == "error")
                .map(|finding| format!("{}:{:?}", finding.path, finding.line))
                .collect();
            if !loud.is_empty() {
                wrong.push(format!(
                    "{cell} under {rule} = \"warn\" still blocked at {}",
                    loud.join(", ")
                ));
            }

            let off = check(&repo, Some(&law_with(&repo, &rule, "off", &desk, &name)));
            assert_ne!(
                off.code, 3,
                "{cell} could not be judged under off\n{}",
                off.stderr
            );
            let under_off = from_rule(&off, &rule);
            if !under_off.is_empty() {
                wrong.push(format!(
                    "{cell} under {rule} = \"off\" still reported {} finding(s)",
                    under_off.len()
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "{} fixtures reported past the level their repository set:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
