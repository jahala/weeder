//! `weed rules` — the catalogue, as weed will apply it.
//!
//! The face and the judgement read the same catalogue and the same defaults, so
//! this test compares what the binary printed against `Config::default()` rather
//! than against a list written out here. A rule whose printed level drifts from
//! the level weed would apply is the drift this test exists to catch.

mod common;

use common::weed_in;
use tempfile::TempDir;
use weed::core::catalogue::{self, Face, Rule};
use weed::core::config::{Config, RuleSetting};

#[test]
fn rules_prints_every_rule_with_its_id_level_face_and_finding() {
    // Not a repository: the catalogue is weed's own, not something it reads
    // out of a working tree.
    let anywhere = TempDir::new().expect("a directory to run in");
    let run = weed_in(anywhere.path(), &["rules"]);
    assert_eq!(run.code, 0);
    assert_eq!(run.stderr, "");

    let lines = run.stdout_lines();
    assert_eq!(
        lines.len(),
        catalogue::rules().len(),
        "one line per rule, and no rule left out"
    );

    let defaults = Config::default();
    for (line, rule) in lines.iter().zip(catalogue::rules()) {
        let cells: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(cells[0], rule.id, "the id comes first, in catalogue order");
        assert_eq!(
            cells[1],
            expected_level(rule, &defaults),
            "{}: the printed level is the level weed would apply",
            rule.id
        );
        assert_eq!(cells[2], expected_face(rule), "{}: the face", rule.id);
        assert!(
            line.ends_with(rule.short_description),
            "{}: the line ends with what the rule reports",
            rule.id
        );
    }
}

#[test]
fn rules_as_json_carries_the_same_catalogue() {
    let anywhere = TempDir::new().expect("a directory to run in");
    let run = weed_in(anywhere.path(), &["rules", "--format", "json"]);
    assert_eq!(run.code, 0);

    let entries: Vec<serde_json::Value> = serde_json::from_str(&run.stdout).expect("json is json");
    assert_eq!(entries.len(), catalogue::rules().len());

    let defaults = Config::default();
    for (entry, rule) in entries.iter().zip(catalogue::rules()) {
        assert_eq!(entry["id"], rule.id);
        assert_eq!(entry["level"], expected_level(rule, &defaults));
        assert_eq!(entry["face"], expected_face(rule));
        assert_eq!(entry["finding"], rule.short_description);
        assert_eq!(entry["description"], rule.full_description);
    }
}

/// What `Config::default()` says this rule's level is, in the word `weed rules`
/// should have printed for it.
fn expected_level(rule: &Rule, defaults: &Config) -> &'static str {
    match defaults.rules.get(rule.id) {
        Some(RuleSetting::Block) => "block",
        Some(RuleSetting::Warn) => "warn",
        Some(RuleSetting::On) => "on",
        Some(RuleSetting::Off) | None => "off",
    }
}

fn expected_face(rule: &Rule) -> &'static str {
    match rule.face {
        Face::Check => "check",
        Face::Scan => "scan",
    }
}
