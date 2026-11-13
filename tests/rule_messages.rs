//! What a block-level finding says, and what it offers to do about itself.
//!
//! Every rule that stops a commit has to earn it in one message: what was found,
//! why it matters, and what to do next, in that order and nothing else. Two of
//! them can also hand back the edit that clears the finding, a marker and a
//! conflict marker are both a line that should not be there, and that arrives
//! as a SARIF fix deleting exactly the line the change added.
//!
//! Where a block-level finding offers a way through, it names the one every
//! face honours: a `Weeder-allow:` trailer on the commit. A marker on the line
//! is read by plain `check` alone, never by a hook and never under `--strict`,
//! so a finding that taught it would send an agent to write something that
//! changes nothing, and then to try again (#68). A warning that asks the change
//! to say why names the same trailer, because that is where a change says it
//! (#89), and so does a finding a warn-level rule blocks by override.

mod common;

use common::{across_cores, fixture, fixture_root, Repo};
use weeder::core::catalogue::{self, Face};
use weeder::core::Level;

/// The words weeder's voice does not use, whatever a rule has found.
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
const BLOCKING: [(&str, &[&str]); 10] = [
    ("T1", &["ts", "py", "rs", "go"]),
    ("T2", &["ts", "py", "rs", "go", "sh"]),
    ("T3", &["ts", "py", "rs", "go"]),
    ("T7", &["ts", "py", "rs", "go"]),
    ("S1", &["ts", "py", "rs", "go", "sh"]),
    ("X1", &["ts", "py", "rs", "go"]),
    ("C1", &["paths"]),
    ("C4", &["ts", "py", "rs", "go"]),
    ("G1", &["ts", "py", "rs", "go"]),
    ("G3", &["ts", "py", "rs", "go"]),
];

/// The rules that also carry the edit that clears the finding.
const FIXED: [&str; 2] = ["T3", "G1"];

#[test]
fn every_block_level_result_says_what_why_and_next() {
    let cells: Vec<(&str, &str)> = BLOCKING
        .iter()
        .flat_map(|&(rule, cases)| cases.iter().map(move |&case| (rule, case)))
        .collect();
    across_cores(&cells, |&(rule, case)| {
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
                    "{rule}/{case}: weeder does not say `{word}`: {message}"
                );
            }
        }
    });
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

/// The warn-level rules whose next action asks the change to say something:
/// why the new expectation is right, why a step moved, why any failure will
/// do, why code nothing runs yet is staged, why a case that checks nothing
/// is the point. A trailer is where a change says it, and it quiets the
/// warning to a note, so the next action names it (#89).
const STATED: [&str; 5] = ["T5", "C3", "T6", "S4", "T9"];

/// The fixtures where a rule that warns by default blocks one finding by
/// override, and the folder each keeps that finding's case in. A finding that
/// blocks names the trailer whatever level its rule defaults to.
const OVERRIDDEN: [(&str, &str, &str); 1] = [("C3", "paths", "promoted")];

#[test]
fn every_block_level_next_names_the_trailer_and_none_teaches_the_line_marker() {
    let blocking: Vec<&str> = catalogue::rules()
        .iter()
        .filter(|rule| rule.face == Face::Check && rule.default_level == Level::Block)
        .map(|rule| rule.id)
        .collect();
    assert!(
        blocking.len() >= BLOCKING.len(),
        "the catalogue blocks on at least the rules this file types by hand"
    );
    for rule in STATED {
        assert!(
            catalogue::rule(rule).is_some_and(|rule| rule.default_level == Level::Warn),
            "{rule} is a warn-level rule, or it belongs with the blocking ones"
        );
    }
    let mut read: Vec<(&str, String, &str, &str, i32)> = Vec::new();
    for (rule, level, code) in blocking
        .into_iter()
        .map(|rule| (rule, "error", 2))
        .chain(STATED.into_iter().map(|rule| (rule, "warning", 0)))
    {
        let cases = fire_cases(rule);
        assert!(
            !cases.is_empty(),
            "{rule}: the rule has a fire fixture to read its message from"
        );
        read.extend(
            cases
                .into_iter()
                .map(|case| (rule, case, "fire", level, code)),
        );
    }
    read.extend(
        OVERRIDDEN
            .into_iter()
            .map(|(rule, case, variant)| (rule, case.to_string(), variant, "error", 2)),
    );

    across_cores(&read, |(rule, case, variant, level, code)| {
        let (rule, variant, level, code) = (*rule, *variant, *level, *code);
        let trailer = format!("`Weeder-allow: {rule}");
        let marker = format!("weeder-allow {rule}");
        let run = fixture(rule, case, variant);
        let reported: Vec<String> = results_exiting(&run, code)
            .iter()
            .filter(|result| result["ruleId"] == rule && result["level"] == level)
            .map(|result| {
                result["message"]["text"]
                    .as_str()
                    .expect("a result carries a message")
                    .to_string()
            })
            .collect();
        assert!(
            !reported.is_empty(),
            "{rule}/{case}/{variant}: the fixture reports its own rule at {level}"
        );
        for message in reported {
            let parts = sentences(&message);
            let next = parts.last().expect("a message has a next action");
            assert!(
                next.contains(&trailer),
                "{rule}/{case}/{variant}: the next action names the {trailer} <reason>` \
                 trailer, the allowance every face reads: {message}"
            );
            assert!(
                !message.contains(&marker),
                "{rule}/{case}/{variant}: no hook and no --strict run honours `{marker}:` \
                 on a line, so the message does not teach it: {message}"
            );
        }
    });
}

/// The fixture folders a rule keeps a fire case in, read off the tree, so a
/// language or a folder added tomorrow is read too.
fn fire_cases(rule: &str) -> Vec<String> {
    let mut cases: Vec<String> = std::fs::read_dir(fixture_root().join(rule))
        .unwrap_or_else(|error| panic!("{rule}: the rule's fixtures should be readable: {error}"))
        .map(|entry| entry.expect("a fixture entry should be readable").path())
        .filter(|path| path.join("fire").is_dir())
        .filter_map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .collect();
    cases.sort();
    cases
}

fn fire(rule: &str, case: &str) -> Repo {
    fixture(rule, case, "fire")
}

fn results(repo: &Repo) -> Vec<serde_json::Value> {
    results_exiting(repo, 2)
}

/// The results of a run that leaves with `code`: 2 for a fire fixture that
/// blocks, 0 for one that warns.
fn results_exiting(repo: &Repo, code: i32) -> Vec<serde_json::Value> {
    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.code, code,
        "a fire fixture leaves with {code}\n{}",
        run.stderr
    );
    run.log()["runs"][0]["results"]
        .as_array()
        .expect("a run carries a results array")
        .clone()
}

/// A message split back into the sentences it was written from. weeder joins what,
/// why and next with a space, and each one ends with a full stop.
fn sentences(message: &str) -> Vec<String> {
    message
        .split(". ")
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}
