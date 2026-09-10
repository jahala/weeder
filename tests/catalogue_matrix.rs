//! The catalogue, walked.
//!
//! Every rule `weeder rules` prints has two fixtures in each of the four
//! languages: a `fire` that is the dishonest change the rule exists for, and a
//! `silent` that is its nearest honest neighbour. This test walks that matrix,
//! replays each fixture through the real binary in a real repository, and holds
//! the answer to the whole result set rather than to the presence of one
//! finding: a fire cell reports the rule under test and nothing else, a silent
//! cell reports nothing at all.
//!
//! Nothing else is the hard half. A fixture written for one rule usually trips a
//! neighbour on the way past, and a matrix that shrugged at that would stop
//! telling a rule that fires from a rule that fires on everything. So each cell
//! runs with a `weeder.toml` that leaves one rule on, built on top of whatever the
//! fixture itself wrote, and `--rules` does the same for the scan face. The
//! configuration is weeder's own, not a switch this test invented.
//!
//! Each face is asked its own question. `check` judges the index against HEAD,
//! `scan` judges a tree that has one state and no diff, and `bite` runs a real
//! suite over a real history, under that language's real runner: node, the
//! interpreter, cargo, go. A machine without one of them cannot answer for that
//! language, and this test says so rather than passing.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;

use tempfile::TempDir;

use common::{fixture, fixture_root, phased_fixture, weeder_in, which, Repo, Run};

/// The languages every rule answers for.
const LANGS: [&str; 4] = ["ts", "py", "rs", "go"];

/// The two cases every cell carries.
const CASES: [&str; 2] = ["fire", "silent"];

/// The command each language's `bite` fixture runs its suite under, and the
/// program that has to be on PATH for it to mean anything. weeder runs whatever
/// the caller names, so these are the four projects' own runners.
const SUITES: [(&str, &str, &str); 4] = [
    ("ts", "node", "node --test"),
    ("py", "python3", "python3 -m unittest -v test_parse"),
    ("rs", "cargo", "cargo test"),
    ("go", "go", "go test ./..."),
];

/// The file a fixture states its own law in, and the file this test writes the
/// one-rule law over.
const CONFIG: &str = "weeder.toml";

/// One rule as `weeder rules` prints it. The catalogue is read from the binary
/// rather than from a list here: a rule that reaches the catalogue and not the
/// fixtures is exactly what this test is for.
#[derive(Debug, Clone)]
struct Rule {
    id: String,
    face: String,
    level: String,
}

impl Rule {
    /// The level a SARIF result carries when this rule reports at its default.
    fn sarif_level(&self) -> &'static str {
        match self.level.as_str() {
            "block" => "error",
            _ => "warning",
        }
    }

    /// What the run leaves with when this rule fires at its default. Only a
    /// block-level result stops a change; `scan` never stops one at all.
    fn firing_code(&self) -> i32 {
        let stops = (self.face == "check" || self.face == "bite") && self.level == "block";
        if stops {
            2
        } else {
            0
        }
    }
}

/// The rules this platform can walk. A `bite` cell runs its language's own
/// suite through `sh`, which Windows has on no PATH: git ships one for its own
/// hooks and puts it nowhere a program finds it. The face is unshipped, so the
/// walk covers the faces that run here and `tests/bite.rs` holds `bite` on unix.
fn walkable(rule: &Rule) -> bool {
    cfg!(unix) || rule.face != "bite"
}

fn catalogue() -> Vec<Rule> {
    let run = weeder_in(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        &["rules", "--format", "json"],
    );
    assert_eq!(run.code, 0, "the catalogue should print: {}", run.stderr);
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(&run.stdout).expect("the catalogue should be json");
    let walked: Vec<Rule> = rows
        .iter()
        .map(|row| Rule {
            id: string(&row["id"]),
            face: string(&row["face"]),
            level: string(&row["level"]),
        })
        .filter(walkable)
        .collect();
    assert!(
        !walked.is_empty(),
        "the catalogue should print rules this platform can walk"
    );
    walked
}

fn string(value: &serde_json::Value) -> String {
    value
        .as_str()
        .expect("the catalogue writes strings")
        .to_string()
}

/// A `weeder.toml` that leaves one rule on and every other rule off, written on
/// top of whatever law the fixture itself wrote, so a cell that needs layers or
/// a scope keeps them.
fn one_rule_config(rule: &Rule, repo: &Repo, catalogue: &[Rule], desk: &TempDir) -> PathBuf {
    let own = std::fs::read_to_string(repo.root().join(CONFIG)).unwrap_or_default();
    assert!(
        !own.contains("[rules]"),
        "a fixture that states its own rule levels cannot be handed another [rules] table"
    );
    let mut text = own;
    text.push_str("\n[rules]\n");
    for other in catalogue {
        let setting = if other.id == rule.id {
            rule.level.as_str()
        } else {
            "off"
        };
        text.push_str(&format!("{} = \"{setting}\"\n", other.id));
    }
    let path = desk.path().join(format!("{}-{CONFIG}", rule.id));
    std::fs::write(&path, text).expect("the one-rule law should be writable");
    path
}

/// The rule ids a run reported, each one once.
fn reported(run: &Run) -> BTreeSet<String> {
    run.findings()
        .into_iter()
        .map(|finding| finding.rule)
        .collect()
}

/// Where a cell's fixture is, so a failure names the directory to go and look at.
fn cell(rule: &Rule, lang: &str, case: &str) -> String {
    format!("fixtures/adversarial/{}/{lang}/{case}", rule.id)
}

/// The command the suite of a `bite` fixture runs under, with the program it
/// needs checked for first: a language whose runner is missing is a language
/// this matrix cannot answer for, and saying nothing about it would be worse
/// than saying so.
fn suite(lang: &str) -> &'static str {
    let (_, program, command) = SUITES
        .iter()
        .find(|(named, _, _)| *named == lang)
        .unwrap_or_else(|| panic!("{lang} has no suite to run its bite fixture under"));
    assert!(
        which(program).is_some(),
        "the {lang} fixture runs a real suite, and {program} is not on PATH"
    );
    command
}

/// One cell, run through the face its rule belongs to.
fn judge(rule: &Rule, lang: &str, case: &str, catalogue: &[Rule], desk: &TempDir) -> Run {
    match rule.face.as_str() {
        "check" => {
            let repo = fixture(&rule.id, lang, case);
            let changed = repo.git(&["diff", "--cached", "--name-only"]);
            assert!(
                !changed.trim().is_empty(),
                "{} stages no change, so neither its noise nor its silence proves anything",
                cell(rule, lang, case)
            );
            let config = one_rule_config(rule, &repo, catalogue, desk);
            repo.weeder(&[
                "check",
                "--format",
                "sarif",
                "--config",
                &config.display().to_string(),
            ])
        }
        "scan" => {
            let repo = fixture(&rule.id, lang, case);
            assert!(
                !repo.git(&["ls-files"]).trim().is_empty(),
                "{} commits no tree for a scan rule to read",
                cell(rule, lang, case)
            );
            repo.weeder(&["scan", "--rules", &rule.id, "--format", "sarif"])
        }
        "bite" => {
            let repo = phased_fixture(&rule.id, lang, case);
            let phases = repo.try_git(&["rev-parse", "--verify", "HEAD~2"]);
            assert_eq!(
                phases.code,
                0,
                "{} needs the three commits a phased node leaves: base, tests, implementation",
                cell(rule, lang, case)
            );
            repo.weeder(&["bite", "--test", suite(lang), "--format", "sarif"])
        }
        other => panic!(
            "{} belongs to a face this test has never met: {other}",
            rule.id
        ),
    }
}

#[test]
fn every_rule_the_catalogue_prints_has_a_fire_and_a_silent_fixture_in_every_language() {
    let rules = catalogue();
    assert!(!rules.is_empty(), "the catalogue should print some rules");

    let mut missing = Vec::new();
    for rule in &rules {
        for lang in LANGS {
            for case in CASES {
                let path = fixture_root().join(&rule.id).join(lang).join(case);
                if !path.join("before").is_dir() {
                    missing.push(cell(rule, lang, case));
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the catalogue prints rules the fixtures do not answer for:\n{}",
        missing.join("\n")
    );
}

#[test]
fn every_fire_fixture_reports_its_own_rule_and_nothing_else() {
    let rules = catalogue();
    let desk = TempDir::new().expect("the one-rule laws need somewhere to sit");
    let mut wrong = Vec::new();

    for rule in &rules {
        for lang in LANGS {
            let run = judge(rule, lang, "fire", &rules, &desk);
            let found = reported(&run);
            let expected: BTreeSet<String> = [rule.id.clone()].into_iter().collect();
            if found != expected {
                wrong.push(format!(
                    "{} reported {:?}, and the cell is for {}\n{}",
                    cell(rule, lang, "fire"),
                    found,
                    rule.id,
                    run.stderr
                ));
                continue;
            }
            for finding in run.findings() {
                assert_eq!(
                    finding.level,
                    rule.sarif_level(),
                    "{} reports {} at the level the catalogue prints",
                    cell(rule, lang, "fire"),
                    rule.id
                );
            }
            assert_eq!(
                run.code,
                rule.firing_code(),
                "{} left with the wrong code for a {} rule\n{}",
                cell(rule, lang, "fire"),
                rule.level,
                run.stderr
            );
        }
    }

    assert!(
        wrong.is_empty(),
        "{} fire cells answered with something other than their own rule:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

#[test]
fn every_silent_fixture_reports_nothing() {
    let rules = catalogue();
    let desk = TempDir::new().expect("the one-rule laws need somewhere to sit");
    let mut spoke = Vec::new();

    for rule in &rules {
        for lang in LANGS {
            let run = judge(rule, lang, "silent", &rules, &desk);
            let found = reported(&run);
            if !found.is_empty() {
                spoke.push(format!(
                    "{} reported {:?}\n{}",
                    cell(rule, lang, "silent"),
                    found,
                    run.stderr
                ));
                continue;
            }
            assert_eq!(
                run.code,
                0,
                "{} found nothing and still left with {}\n{}",
                cell(rule, lang, "silent"),
                run.code,
                run.stderr
            );
        }
    }

    assert!(
        spoke.is_empty(),
        "{} silent cells reported something:\n{}",
        spoke.len(),
        spoke.join("\n")
    );
}
