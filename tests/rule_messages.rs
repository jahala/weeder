//! What a block-level finding says, and what it offers to do about itself.
//!
//! Every rule that stops a commit has to earn it in one message: what was found,
//! why it matters, and what to do next, in that order and nothing else. Two of
//! them can also hand back the edit that clears the finding, a marker and a
//! conflict marker are both a line that should not be there, and that arrives
//! as a SARIF fix deleting exactly the line the change added.

mod common;

use common::{fixture, Repo};

/// The words weed's voice does not use, whatever a rule has found.
const NEVER: [&str; 12] = [
    "supercharge",
    "unlock",
    "10x",
    "magic",
    "synergy",
    "revolutionary",
    "game-changing",
    "cutting-edge",
    "seamless",
    "effortless",
    "next-gen",
    "ai-powered",
];

/// The rules that block by default, and the fixture folders that make each one
/// fire. A rule proven in four languages is read in all four.
const BLOCKING: [(&str, &[&str]); 8] = [
    ("T1", &["ts", "py", "rs", "go"]),
    ("T2", &["ts", "py", "rs", "go"]),
    ("T3", &["ts", "py", "rs", "go"]),
    ("T7", &["ts", "py", "rs", "go"]),
    ("S1", &["ts", "py", "rs", "go"]),
    ("X1", &["ts", "py", "rs", "go"]),
    ("C1", &["paths"]),
    ("G1", &["ts", "py", "rs", "go"]),
];

/// The rules that also carry the edit that clears the finding.
const FIXED: [&str; 2] = ["T3", "G1"];

#[test]
fn every_block_level_result_says_what_why_and_next() {
    for (rule, cases) in BLOCKING {
        for case in cases {
            let run = fire(rule, case);
            let results = results(&run);
            assert!(
                !results.is_empty(),
                "{rule}/{case}: the fire fixture has to produce a finding"
            );
            for result in &results {
                let message = result["message"]["text"]
                    .as_str()
                    .expect("a result carries a message")
                    .to_string();
                let parts = sentences(&message);
                assert_eq!(
                    parts.len(),
                    3,
                    "{rule}/{case}: what, why and next, and nothing else: {message}"
                );
                for part in &parts {
                    assert!(
                        part.len() > 10,
                        "{rule}/{case}: every part says something: {message}"
                    );
                }
                assert!(
                    parts[0] != parts[1] && parts[1] != parts[2] && parts[0] != parts[2],
                    "{rule}/{case}: three sentences, not one repeated: {message}"
                );
                assert!(
                    message.ends_with('.'),
                    "{rule}/{case}: the message ends where it stops: {message}"
                );
                assert!(
                    !message.starts_with(char::is_uppercase),
                    "{rule}/{case}: sentence case: {message}"
                );
                let lowered = message.to_ascii_lowercase();
                for word in NEVER {
                    assert!(
                        !lowered.contains(word),
                        "{rule}/{case}: weed does not say `{word}`: {message}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_marker_and_a_conflict_marker_carry_the_edit_that_takes_the_line_back_out() {
    let mut proven = Vec::new();
    for (rule, cases) in BLOCKING {
        if !FIXED.contains(&rule) {
            continue;
        }
        proven.push(rule);
        for case in cases {
            let run = fire(rule, case);
            for result in results(&run) {
                let fixes = result
                    .get("fixes")
                    .and_then(|fixes| fixes.as_array())
                    .unwrap_or_else(|| panic!("{rule}/{case}: an added line has a fix: {result}"));
                assert_eq!(fixes.len(), 1, "{rule}/{case}: one fix, one line");

                let change = &fixes[0]["artifactChanges"][0];
                assert_eq!(
                    change["artifactLocation"]["uri"],
                    result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
                    "{rule}/{case}: the fix edits the file the finding names"
                );
                let replacement = &change["replacements"][0];
                assert_eq!(
                    replacement["deletedRegion"]["startLine"],
                    result["locations"][0]["physicalLocation"]["region"]["startLine"],
                    "{rule}/{case}: the fix deletes the line the finding landed on"
                );
                assert!(
                    replacement.get("insertedContent").is_none(),
                    "{rule}/{case}: taking the line out puts nothing back: {replacement}"
                );
                assert!(
                    fixes[0]["description"]["text"]
                        .as_str()
                        .is_some_and(|text| text.ends_with('.')),
                    "{rule}/{case}: the fix says what it does"
                );
            }
        }
    }
    assert_eq!(proven, FIXED, "both rules that carry a fix are read");
}

fn fire(rule: &str, case: &str) -> Repo {
    fixture(rule, case, "fire")
}

fn results(repo: &Repo) -> Vec<serde_json::Value> {
    let run = repo.weed(&["check"]);
    assert_eq!(run.code, 2, "a fire fixture blocks\n{}", run.stderr);
    run.log()["runs"][0]["results"]
        .as_array()
        .expect("a run carries a results array")
        .clone()
}

/// A message split back into the sentences it was written from. weed joins what,
/// why and next with a space, and each one ends with a full stop.
fn sentences(message: &str) -> Vec<String> {
    message
        .split(". ")
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}
