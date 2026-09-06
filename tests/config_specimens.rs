//! `[scope] specimens`, the paths weed is told not to judge.
//!
//! An adversarial fixture is written to look dishonest: that is what makes it a
//! fixture. A repository that keeps such files has to be able to say so, and
//! `[scope] specimens` is the one sentence it says it in. What it can say is
//! bounded: directories under `fixtures/adversarial/`, never a rule, never a
//! path that reaches somewhere else, because an exclusion nobody can see is a
//! rule turned off in the dark. Every path an exclusion covers is still
//! reported, once, at note level.
//!
//! The repository below carries the same dishonest file twice, once under the
//! fixture root and once in `src/`, so every test here turns on the exclusion
//! and nothing else.

mod common;

use common::{ours, separator, theirs, Finding, Repo, BRANCH};

/// Where a specimen exclusion may point, and the only place it may.
const SPECIMEN: &str = "fixtures/adversarial/X1/ts/fire/after/src/format.test.ts";

/// The same file, in the source tree, where no exclusion can reach it.
const SOURCE: &str = "src/format.test.ts";

/// A test file carrying a skip marker and a conflict git could not merge: two
/// rules, both blocking, on one path. One finding on this path is the claim,
/// not one finding per rule.
fn dishonest_test() -> String {
    format!(
        "import {{ format }} from \"./format\";\n\
         \n\
         describe(\"format\", () => {{\n\
         \x20 it.skip(\"pads to the width\", () => {{\n\
         {opener}\n\
         \x20   expect(format(\"a\", 3)).toBe(\"a  \");\n\
         {separator}\n\
         \x20   expect(format(\"a\", 3)).toBe(\"a\");\n\
         {closer}\n\
         \x20 }});\n\
         }});\n",
        opener = ours("HEAD"),
        separator = separator(),
        closer = theirs(BRANCH),
    )
}

/// A repository holding that file in both places, staged and unjudged, with
/// whatever `weed.toml` the test wants it to state.
fn repository(config: Option<&str>) -> Repo {
    let repo = Repo::init();
    repo.write(
        "src/format.ts",
        "export const format = (text: string) => text;\n",
    );
    repo.commit("the state the change starts from");
    if let Some(config) = config {
        repo.write("weed.toml", config);
    }
    repo.write(SPECIMEN, &dishonest_test());
    repo.write(SOURCE, &dishonest_test());
    repo.stage_all();
    repo
}

/// A `weed.toml` stating one specimen entry and nothing else.
fn stating(entry: &str) -> String {
    format!("[scope]\nspecimens = [\"{entry}\"]\n")
}

fn on(found: &[Finding], path: &str) -> Vec<Finding> {
    found
        .iter()
        .filter(|finding| finding.path == path)
        .cloned()
        .collect()
}

fn rules(found: &[Finding]) -> Vec<String> {
    let mut ids: Vec<String> = found.iter().map(|finding| finding.rule.clone()).collect();
    ids.sort();
    ids.dedup();
    ids
}

#[test]
fn without_an_exclusion_the_specimen_is_judged_like_anything_else() {
    let repo = repository(None);
    let run = repo.weed(&["check", "--format", "sarif"]);

    assert_eq!(run.code, 2, "the file blocks: {}", run.stderr);
    let found = run.findings();
    let specimen = on(&found, SPECIMEN);
    assert!(
        rules(&specimen).len() > 1,
        "the specimen fires more than one rule, or this file proves nothing about skipping every rule: {specimen:#?}"
    );
    assert!(
        specimen.iter().any(|finding| finding.level == "error"),
        "and at least one of them blocks: {specimen:#?}"
    );
}

#[test]
fn a_specimen_is_skipped_by_every_rule_and_reported_once_at_note_level() {
    let repo = repository(Some(&stating("fixtures/adversarial/X1")));
    let run = repo.weed(&["check", "--format", "sarif"]);
    let found = run.findings();

    let specimen = on(&found, SPECIMEN);
    assert_eq!(
        specimen.len(),
        1,
        "an excluded path is reported once, whatever the rules would have said: {specimen:#?}"
    );
    assert_eq!(
        specimen[0].level, "note",
        "an exclusion is as visible as an allowance, and stops nothing"
    );
    assert!(
        specimen[0].message.contains("specimens"),
        "the note names the key that excluded the path: {}",
        specimen[0].message
    );

    let source = on(&found, SOURCE);
    assert!(
        rules(&source).len() > 1,
        "the same file in src/ is judged by every rule as before: {source:#?}"
    );
    assert_eq!(
        run.code, 2,
        "and the change is still stopped by what the exclusion does not cover"
    );
}

#[test]
fn a_directory_under_the_entry_is_covered_and_a_sibling_is_not() {
    let repo = repository(Some(&stating("fixtures/adversarial/X1/ts")));
    let run = repo.weed(&["check", "--format", "sarif"]);
    assert_eq!(
        on(&run.findings(), SPECIMEN).len(),
        1,
        "a nested path is inside the directory named"
    );

    let repo = repository(Some(&stating("fixtures/adversarial/T3")));
    let run = repo.weed(&["check", "--format", "sarif"]);
    let specimen = on(&run.findings(), SPECIMEN);
    assert!(
        rules(&specimen).len() > 1,
        "an entry naming another directory covers nothing here: {specimen:#?}"
    );
}

#[test]
fn an_entry_that_covers_nothing_reports_nothing() {
    let repo = repository(Some(&stating("fixtures/adversarial/nothing-is-here")));
    let run = repo.weed(&["check", "--format", "sarif"]);
    let notes: Vec<Finding> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.level == "note")
        .collect();
    assert!(
        notes.is_empty(),
        "weed reports the paths an exclusion covered, and this one covered none: {notes:#?}"
    );
}

#[test]
fn an_excluded_path_is_not_evidence_a_rule_reads_about_another_file() {
    // C2 asks what an added ignore pattern hides, and answers from the paths
    // the repository holds. A specimen is not one of them: a rule that skips a
    // file and then reports another file because of it has not skipped it.
    let ignoring = |config: Option<&str>| {
        let repo = Repo::init();
        repo.write(SPECIMEN, &dishonest_test());
        repo.commit("the specimen is in the repository");
        if let Some(config) = config {
            repo.write("weed.toml", config);
        }
        repo.write(".gitignore", "fixtures/\n");
        repo.stage_all();
        let run = repo.weed(&["check", "--format", "sarif"]);
        rules(&on(&run.findings(), ".gitignore"))
    };

    assert!(
        ignoring(None).contains(&"C2".to_string()),
        "the pattern hides a test file, and weed says so"
    );
    assert!(
        !ignoring(Some(&stating("fixtures/adversarial/X1"))).contains(&"C2".to_string()),
        "the only file it hides is one no rule reads"
    );
}

#[test]
fn scan_skips_the_same_paths_and_says_so_once() {
    // R3 dates a line through git, so the marker below is old enough to report
    // wherever it sits, and the exclusion is the only reason one of them is quiet.
    let repo = Repo::init();
    let marked = "// TODO: split the record on the separator the header names\nexport const parse = (line: string) => line.split(\",\");\n";
    repo.write("weed.toml", &stating("fixtures/adversarial/R3"));
    repo.write("src/parse.ts", marked);
    repo.write(
        "fixtures/adversarial/R3/ts/fire/before/src/parse.ts",
        marked,
    );
    repo.commit_dated("the markers are written", "@1600000000 +0000");

    let run = repo.weed(&["scan", "--format", "sarif"]);
    assert_eq!(run.code, 0, "a scan never blocks: {}", run.stderr);
    let found = run.findings();

    let specimen = on(
        &found,
        "fixtures/adversarial/R3/ts/fire/before/src/parse.ts",
    );
    assert_eq!(
        specimen.len(),
        1,
        "one note, and no scan rule read the file: {specimen:#?}"
    );
    assert_eq!(specimen[0].level, "note");

    let source = on(&found, "src/parse.ts");
    assert!(
        rules(&source).contains(&"R3".to_string()),
        "the marker outside the exclusion is still reported: {source:#?}"
    );
}

#[test]
fn an_entry_outside_the_fixture_root_is_refused() {
    for entry in ["src/core", "docs", "/etc"] {
        let repo = repository(Some(&stating(entry)));
        let run = repo.weed(&["check", "--format", "sarif"]);
        assert_eq!(
            run.code, 3,
            "an exclusion weed cannot allow is a run that never happened: {entry}"
        );
        assert!(
            run.stderr.contains(entry),
            "the refusal names the entry it refused: {entry}, and weed said: {}",
            run.stderr
        );
        assert!(
            run.stderr.contains("fixtures/adversarial"),
            "and says where an exclusion may point: {}",
            run.stderr
        );
    }
}

#[test]
fn a_glob_that_reaches_past_the_root_is_refused() {
    for entry in [
        "fixtures/adversarial/../src",
        "fixtures/*",
        "fixtures/adversarial*/ts",
        "**/*.ts",
    ] {
        let repo = repository(Some(&stating(entry)));
        let run = repo.weed(&["check", "--format", "sarif"]);
        assert_eq!(
            run.code, 3,
            "a glob that can name a file outside the fixture root is refused: {entry}"
        );
        assert!(
            run.stderr.contains(entry),
            "the refusal names the glob it refused: {entry}, and weed said: {}",
            run.stderr
        );
    }
}

#[test]
fn a_rule_name_in_the_list_is_refused_as_the_rule_it_is() {
    for entry in ["T3", "x1"] {
        let repo = repository(Some(&stating(entry)));
        let run = repo.weed(&["check", "--format", "sarif"]);
        assert_eq!(
            run.code, 3,
            "specimens excludes paths; a rule turned off there would be a rule turned off in the dark: {entry}"
        );
        assert!(
            run.stderr.contains(entry),
            "the refusal names what was written: {entry}, and weed said: {}",
            run.stderr
        );
        assert!(
            run.stderr.contains("[rules]"),
            "and says where a rule's level is stated instead: {}",
            run.stderr
        );
    }
}

#[test]
fn a_glob_inside_the_root_is_allowed() {
    let repo = repository(Some(&stating("fixtures/adversarial/**/after")));
    let run = repo.weed(&["check", "--format", "sarif"]);
    let specimen = on(&run.findings(), SPECIMEN);
    assert_eq!(
        specimen.len(),
        1,
        "a glob that cannot leave the fixture root is a specimen exclusion like any other: {specimen:#?}"
    );
    assert_eq!(specimen[0].level, "note");
}

#[test]
fn scan_refuses_the_same_config_check_refuses() {
    let repo = repository(Some(&stating("src/**")));
    let run = repo.weed(&["scan", "--format", "sarif"]);
    assert_eq!(
        run.code, 3,
        "a scan that could not read the config never ran, and says so"
    );
    assert!(
        run.stderr.contains("src/**"),
        "naming the entry: {}",
        run.stderr
    );
}
