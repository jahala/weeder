//! `weeder rules`, the catalogue, as weeder will apply it.
//!
//! The face and the judgement read the same catalogue and the same defaults, so
//! this test compares what the binary printed against `Config::default()` rather
//! than against a list written out here. A rule whose printed level drifts from
//! the level weeder would apply is the drift this test exists to catch.

mod common;

use common::weeder_in;
use tempfile::TempDir;
use weeder::core::catalogue::{self, Face, Rule};
use weeder::core::config::{Config, RuleSetting};

#[test]
fn rules_prints_every_rule_with_its_id_level_face_and_finding() {
    // Not a repository: the catalogue is weeder's own, not something it reads
    // out of a working tree.
    let anywhere = TempDir::new().expect("a directory to run in");
    let run = weeder_in(anywhere.path(), &["rules"]);
    assert_eq!(run.code, 0);
    assert_eq!(run.stderr, "");

    let lines = run.stdout_lines();
    assert_eq!(
        lines.len(),
        catalogue::rules().len() + 1,
        "a header, then one line per rule, and no rule left out"
    );

    let defaults = Config::default();
    for (line, rule) in lines.iter().skip(1).zip(catalogue::rules()) {
        let cells: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(cells[0], rule.id, "the id comes first, in catalogue order");
        assert_eq!(
            cells[1],
            expected_level(rule, &defaults),
            "{}: the printed level is the level weeder would apply",
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

/// The columns `weeder rules` prints, named as the json names them. Without
/// them the fourth column, what a rule may reach over the network, reads as
/// whatever a reader guesses it is, and one reader took its `none` for "this
/// rule takes no allowance" (#106).
const COLUMNS: [&str; 5] = ["id", "level", "face", "network", "finding"];

#[test]
fn rules_names_its_columns_above_the_rows() {
    let anywhere = TempDir::new().expect("a directory to run in");
    let run = weeder_in(anywhere.path(), &["rules"]);
    assert_eq!(run.code, 0);
    let lines = run.stdout_lines();
    let header: Vec<&str> = lines
        .first()
        .expect("the table prints something")
        .split_whitespace()
        .collect();
    assert_eq!(
        header, COLUMNS,
        "the first line names every column, in order"
    );

    // Each name sits over its column: a row's cell starts where its name does.
    let row = lines.get(1).expect("a row under the header");
    let starts = |line: &str| -> Vec<usize> {
        let mut found = Vec::new();
        let characters: Vec<char> = line.chars().collect();
        for (at, character) in characters.iter().enumerate() {
            let opens = !character.is_whitespace()
                && (at == 0 || (at >= 2 && characters[at - 1] == ' ' && characters[at - 2] == ' '));
            if opens {
                found.push(at);
            }
        }
        found
    };
    assert_eq!(
        starts(lines[0]),
        starts(row),
        "the header is aligned with the rows:\n{}\n{row}",
        lines[0]
    );

    let json = weeder_in(anywhere.path(), &["rules", "--format", "json"]);
    let entries: Vec<serde_json::Value> = serde_json::from_str(&json.stdout).expect("json is json");
    let keys: Vec<String> = entries[0]
        .as_object()
        .expect("an entry is an object")
        .keys()
        .cloned()
        .collect();
    for column in COLUMNS {
        assert!(
            keys.iter().any(|key| key == column),
            "the table's `{column}` is the json's `{column}`: {keys:?}"
        );
    }
}

#[test]
fn rules_as_json_carries_the_same_catalogue() {
    let anywhere = TempDir::new().expect("a directory to run in");
    let run = weeder_in(anywhere.path(), &["rules", "--format", "json"]);
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

/// What `Config::default()` says this rule's level is, in the word `weeder rules`
/// should have printed for it.
fn expected_level(rule: &Rule, defaults: &Config) -> &'static str {
    match defaults.setting(rule) {
        RuleSetting::Block => "block",
        RuleSetting::Warn => "warn",
        RuleSetting::On => "on",
        RuleSetting::Off => "off",
    }
}

fn expected_face(rule: &Rule) -> &'static str {
    match rule.face {
        Face::Check => "check",
        Face::Scan => "scan",
        Face::Bite => "bite",
    }
}
