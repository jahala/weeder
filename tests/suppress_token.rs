//! The tokens an allowance is written in, and the one spelling the gate reads.
//!
//! weeder was called weed until 2026-09-06, and its suppression tokens carried
//! the old name for two days after the binary stopped. They rename here:
//! `Weeder-allow:` in a commit message, `weeder-allow <RULE>:` on a line. The
//! gate honours those and nothing else, so the spelling it carried before means
//! exactly what any other sentence in a commit message means, which is nothing.
//! Reading both would be a shim, and weeder does not ship one.
//!
//! What a person has to be told is the other half. A token nobody can find is a
//! gate with no door, so the finding that reports a rule teaches the spelling
//! that clears it, and so do the pages an agent reads before it ever sees one.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use common::{conflicted_parser, fixture, fixture_root, ours, separator, theirs, Repo, BRANCH};

const REASON: &str = "the merge finishes in the follow-up commit";

/// The spelling the tool carried before it was renamed, built rather than
/// written, so that this file is not itself a place the old token lives on.
fn retired() -> (String, String) {
    let stem = "weed";
    (format!("W{}-allow:", &stem[1..]), format!("{stem}-allow"))
}

/// The fixture directories that are not a rule's: the audit keeps its case
/// packets and its session records under the same root, and those are a
/// record of what an auditor was handed rather than a change to judge.
const NOT_A_RULE: [&str; 1] = ["calibration-audit"];

#[test]
fn a_trailer_in_the_tool_s_own_name_suppresses() {
    let repo = fixture("G1", "ts", "fire");
    repo.pending_message(&format!(
        "Split on semicolons\n\nWeeder-allow: G1 {REASON}\n"
    ));

    let run = repo.weeder(&["check"]);
    let findings = run.findings();
    assert!(!findings.is_empty(), "the rule still reports");
    assert!(
        findings.iter().all(|finding| finding.suppressed),
        "the trailer the gate reads carries the reason into the log"
    );
    assert_eq!(run.code, 0, "an allowance weeder read stops nobody");
}

#[test]
fn the_retired_trailer_neither_suppresses_nor_draws_a_complaint() {
    let (trailer, _) = retired();
    let repo = fixture("G1", "ts", "fire");
    repo.pending_message(&format!("Split on semicolons\n\n{trailer} G1 {REASON}\n"));

    let run = repo.weeder(&["check"]);
    assert!(
        run.findings().iter().all(|finding| !finding.suppressed),
        "the spelling the gate no longer reads allows nothing"
    );
    assert_eq!(run.code, 2, "the findings stand");
    assert_eq!(
        run.stderr, "",
        "a line weeder does not read is a line weeder has nothing to say about"
    );
}

#[test]
fn an_inline_marker_in_the_tool_s_own_name_suppresses() {
    let repo = Repo::init();
    repo.write(
        "src/parser.ts",
        &conflicted_parser(Some(&format!("// weeder-allow G1: {REASON}"))),
    );
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    let findings = run.findings();
    assert_eq!(findings.len(), 3, "one finding per marker");
    assert!(findings.iter().all(|finding| finding.suppressed));
    assert_eq!(run.code, 0);
}

#[test]
fn the_retired_inline_marker_is_neither_honoured_nor_complained_about() {
    let (_, marker) = retired();
    let repo = Repo::init();
    repo.write(
        "src/parser.ts",
        &conflicted_parser(Some(&format!("// {marker} G1: {REASON}"))),
    );
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    assert!(
        run.findings().iter().all(|finding| !finding.suppressed),
        "the spelling the gate no longer reads allows nothing"
    );
    assert_eq!(run.code, 2);
    assert_eq!(run.stderr, "", "and it is not a malformed one either");

    // The same line under --strict is a comment like any other, so the run
    // reaches a judgement rather than refusing to make one.
    let strict = repo.weeder(&["check", "--strict"]);
    assert_eq!(
        strict.code, 2,
        "a comment is not something weeder cannot read"
    );
}

#[test]
fn a_marker_with_no_reason_is_complained_about_in_the_spelling_that_works() {
    let repo = Repo::init();
    repo.write(
        "src/parser.ts",
        &format!(
            "export function parse(input: string): string[] {{\n\
             \x20 // weeder-allow G1\n\
             {opener}\n  return input.split(\",\");\n\
             {separator}\n  return input.split(\";\");\n\
             {closer}\n}}\n",
            opener = ours("HEAD"),
            separator = separator(),
            closer = theirs(BRANCH),
        ),
    );
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    assert_eq!(run.code, 2, "the findings still stand");
    let complaint = run
        .stderr_lines()
        .first()
        .map(ToString::to_string)
        .expect("a suppression weeder could not read is said out loud");
    assert!(
        complaint.contains("weeder-allow G1: the reason"),
        "the complaint teaches the spelling that would have worked: {complaint}"
    );
}

/// Every message the binary can print, gathered by replaying each rule's own
/// dishonest change through the face that judges it. A rule that teaches an
/// allowance has to teach the one the gate reads, and a rule added tomorrow
/// brings its fixtures with it and is swept up here too.
#[test]
fn no_finding_weeder_prints_teaches_a_token_the_gate_no_longer_reads() {
    let (trailer, marker) = retired();
    let mut messages = BTreeSet::new();
    for (rule, face) in catalogue() {
        for lang in languages(&rule) {
            let repo = fixture(&rule, &lang, "fire");
            let run = repo.weeder(&[face.as_str(), "--format", "sarif"]);
            messages.extend(run.findings().into_iter().map(|finding| finding.message));
        }
    }

    assert!(
        !messages.is_empty(),
        "the fixtures produced no message at all, so this proves nothing"
    );
    let taught: Vec<&String> = messages
        .iter()
        .filter(|message| message.contains("weeder-allow") || message.contains("Weeder-allow"))
        .collect();
    assert!(
        !taught.is_empty(),
        "no finding teaches an allowance, so there is nothing here to be right about"
    );
    for message in &messages {
        let retired_marker = message.contains(&marker) && !message.contains("weeder-allow");
        assert!(
            !message.contains(&trailer) && !retired_marker,
            "a finding teaches a token the gate no longer reads: {message}"
        );
    }
}

/// The pages somebody reads before weeder ever prints at them.
#[test]
fn the_pages_an_agent_reads_teach_the_token_the_gate_reads() {
    let (trailer, marker) = retired();
    for page in ["SKILL.md", "docs/rules.md", "README.md"] {
        let text = read(page);
        assert!(
            text.contains("Weeder-allow") || text.contains("weeder-allow"),
            "{page} never names the token an allowance is written in"
        );
        assert!(
            !text.contains(&trailer),
            "{page} still teaches the trailer the gate no longer reads"
        );
        for line in text.lines() {
            assert!(
                !line.contains(&marker) || line.contains("weeder-allow"),
                "{page} still teaches the marker the gate no longer reads: {line}"
            );
        }
    }
}

/// What the repository states about itself is written in the same token. weeder
/// judges its own tree, so an allowance here that the gate cannot read is a
/// finding weeder would report against itself.
#[test]
fn weeder_s_own_allowances_are_written_in_the_token_the_gate_reads() {
    let (trailer, marker) = retired();
    let config = read("weeder.toml");
    assert!(
        config.contains("weeder-allow C1"),
        "weeder.toml allows its own root commit past C1, in the spelling the gate reads"
    );
    assert!(!config.contains(&trailer));
    for line in config.lines() {
        assert!(!line.contains(&marker) || line.contains("weeder-allow"));
    }
}

/// The reason the tokens carry this name, kept beside the parser that reads
/// them. It is the record of what the rename cost and of what stops it costing
/// that again, and a later reader who does not find it there will rename them
/// back.
#[test]
fn the_reason_beside_the_parser_records_what_the_re_seal_cost() {
    let source = read("src/core/suppress.rs");
    let reason: String = source
        .lines()
        .take_while(|line| line.starts_with("//!"))
        .collect::<Vec<&str>>()
        .join(" ");
    assert!(
        !reason.is_empty(),
        "the parser carries no reason at all beside it"
    );
    for said in [
        "forty",
        "seal",
        "wording",
        "Weeder-allow:",
        "weeder-allow <RULE>:",
    ] {
        assert!(
            reason.contains(said),
            "the reason beside the parser never says `{said}`: {reason}"
        );
    }
}

/// Every rule the binary carries and the face that judges it, read from the
/// binary rather than from a list written down here: a rule that reaches the
/// catalogue is a rule whose messages this sweep has to read.
fn catalogue() -> Vec<(String, String)> {
    let run = common::weeder_in(&root(), &["rules", "--format", "json"]);
    assert_eq!(run.code, 0, "the catalogue should print: {}", run.stderr);
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(&run.stdout).expect("the catalogue should be json");
    rows.iter()
        .map(|row| {
            (
                row["id"].as_str().expect("a rule has an id").to_string(),
                row["face"].as_str().expect("a rule has a face").to_string(),
            )
        })
        // `bite` runs a real suite under a real runner, which is the matrix's
        // question rather than this one's. Its fixtures are replayed there.
        .filter(|(_, face)| face != "bite")
        .collect()
}

/// The languages a rule carries a dishonest change in.
fn languages(rule: &str) -> Vec<String> {
    assert!(
        !NOT_A_RULE.contains(&rule),
        "{rule} names a fixture root that is not a rule's"
    );
    let at = fixture_root().join(rule);
    assert!(at.is_dir(), "{rule} is in the catalogue with no fixtures");
    directories(&at)
        .into_iter()
        .filter(|lang| at.join(lang).join("fire").is_dir())
        .collect()
}

fn directories(at: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(at)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", at.display()))
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    found.sort();
    found
}

fn read(path: &str) -> String {
    let file = root().join(path);
    std::fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", file.display()))
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// A line that names the marker, and does not write one. weeder's own pages
/// explain the token, its own source holds it in a constant, and its own tests
/// spell lines that carry it; every one of those is a sentence about an
/// allowance rather than an allowance, and a judge that cannot tell the two
/// apart refuses to read its own documentation.
#[test]
fn a_line_that_names_the_marker_without_writing_one_is_not_an_allowance() {
    let repo = Repo::init();
    repo.write(
        "docs/allowances.md",
        "An allowance is a `weeder-allow` comment beside the line, or a\n\
         `weeder-allow <RULE>: <reason>` where the rule is named.\n",
    );
    repo.write(
        "src/token.rs",
        "/// A `weeder-allow` with no reason is nothing to allow against.\n\
         pub const MARKER: &str = \"weeder-allow\";\n",
    );
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.stderr, "",
        "a page about the marker is not a marker weeder could not read"
    );
    assert_eq!(run.code, 2, "and the conflict markers still stand");

    let strict = repo.weeder(&["check", "--strict"]);
    assert_eq!(
        strict.code, 2,
        "under --strict weeder still reaches a judgement rather than refusing one: {}",
        strict.stderr
    );
}
