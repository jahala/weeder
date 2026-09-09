//! R1, the docs cite something that no longer exists.
//!
//! Four things a document can cite and four authorities that answer for them:
//! the tree for a path, the command's own help for a subcommand and for a flag,
//! and the code for a symbol. Each fixture is a repository whose documentation
//! is true about three of them and wrong about one, so a green run here means
//! weeder found that one rather than that it complained about everything.
//!
//! The command fixtures cite `weeder` itself, and the binary under test is put on
//! PATH in front of everything else, so the help R1 reads is the help this
//! build prints.
//!
//! A citation carries more than a path. The line a document points at is part of
//! what it claims, so a path that resolves is still wrong when the line it names
//! is past the end of the file, and R1 says how long the file is. And a bare name
//! is only as strong as what stands next to it: a paragraph that pins a name to a
//! file or a directory is making a claim about that place, and R1 answers it at
//! warning level naming the place; a name with nothing anchoring it is answered
//! against the whole tree and reported as a note, because prose is full of words
//! that are nobody's symbol.

mod common;

use common::{fixture, fixture_file, path_with_weeder, Finding};

/// R1's findings on a fixture, with weeder on PATH so a cited command resolves.
fn findings(lang: &str, case: &str) -> Vec<Finding> {
    let repo = fixture("R1", lang, case);
    let run = repo.weeder_with(
        &["scan", "--rules", "R1", "--format", "sarif"],
        &[("PATH", &path_with_weeder())],
    );
    assert_eq!(
        run.code, 0,
        "a scan never blocks, and R1/{lang}/{case} left with {}: {}",
        run.code, run.stderr
    );
    run.findings()
}

/// The one finding a fire fixture is built to produce.
fn only(lang: &str) -> Finding {
    let found = findings(lang, "fire");
    assert_eq!(
        found.len(),
        1,
        "R1/{lang}/fire should report the one citation that is wrong, and reported: {found:#?}"
    );
    let finding = found.into_iter().next().expect("one finding");
    assert_eq!(finding.rule, "R1");
    assert!(
        finding.level == "warning" || finding.level == "note",
        "a scan finding never blocks, and this one is a {}",
        finding.level
    );
    assert_eq!(finding.path, "README.md");
    finding
}

/// The finding of a fire fixture whose message names a citation.
fn about(found: &[Finding], cited: &str) -> Finding {
    found
        .iter()
        .find(|finding| finding.message.contains(cited))
        .unwrap_or_else(|| panic!("no finding named `{cited}`: {found:#?}"))
        .clone()
}

/// How many lines a fixture's file carries, counted from the fixture itself so
/// the expectation moves when the file does.
fn lines_of(lang: &str, case: &str, path: &str) -> usize {
    fixture_file("R1", lang, case, &format!("before/{path}"))
        .lines()
        .count()
}

#[test]
fn a_cited_path_the_tree_no_longer_holds_is_reported() {
    let finding = only("paths");
    assert_eq!(finding.level, "warning");
    assert!(
        finding.message.contains("docs/parser.md"),
        "the finding should name the path that is gone: {}",
        finding.message
    );
}

#[test]
fn a_cited_subcommand_the_command_no_longer_offers_is_reported() {
    let finding = only("commands");
    assert_eq!(finding.level, "warning");
    assert!(
        finding.message.contains("prune"),
        "the finding should name the subcommand that is gone: {}",
        finding.message
    );
}

#[test]
fn a_cited_flag_the_command_no_longer_prints_is_reported() {
    let finding = only("flags");
    assert_eq!(finding.level, "warning");
    assert!(
        finding.message.contains("--deep"),
        "the finding should name the flag that is gone: {}",
        finding.message
    );
}

#[test]
fn a_cited_symbol_the_code_no_longer_declares_is_reported() {
    let finding = only("symbols");
    assert!(
        finding.message.contains("renderOutput"),
        "the finding should name the symbol that is gone: {}",
        finding.message
    );
    assert_eq!(
        finding.level, "note",
        "nothing in that paragraph pins the name to a file, so it is the quiet half: {}",
        finding.message
    );
}

#[test]
fn a_cited_line_past_the_end_of_a_file_that_resolves_is_reported() {
    let finding = only("lines");
    assert_eq!(finding.level, "warning");
    assert!(
        finding.message.contains("src/parser.ts:40"),
        "the citation keeps the line it carries: {}",
        finding.message
    );
    let count = lines_of("lines", "fire", "src/parser.ts");
    assert!(
        finding.message.contains(&format!("{count} lines")),
        "the finding should say how long the file is, and it is {count} lines: {}",
        finding.message
    );
}

#[test]
fn a_name_the_paragraph_pins_to_a_file_is_a_warning_that_names_the_file() {
    let found = findings("anchors", "fire");
    assert_eq!(
        found.len(),
        2,
        "one anchored name and one loose one, and R1 reported: {found:#?}"
    );

    let anchored = about(&found, "renderOutput");
    assert_eq!(
        anchored.level, "warning",
        "a name pinned to a file is answered by that file: {}",
        anchored.message
    );
    assert!(
        anchored.message.contains("src/parser.ts"),
        "the finding should name the file the paragraph pinned it to: {}",
        anchored.message
    );

    let loose = about(&found, "runFixture");
    assert_eq!(
        loose.level, "note",
        "a heading ends the anchor, so the name below it stands on its own: {}",
        loose.message
    );
    assert!(
        !loose.message.contains("src/parser.ts"),
        "an unanchored name is answered by the tree and names no file: {}",
        loose.message
    );
}

#[test]
fn a_name_a_directory_anchors_is_judged_against_everything_under_it() {
    let found = findings("directory", "fire");
    assert_eq!(
        found.len(),
        1,
        "the two names under `src/` resolve there and the third does not: {found:#?}"
    );
    let finding = about(&found, "renderOutput");
    assert_eq!(finding.level, "warning");
    assert!(
        finding.message.contains("directory `src/`"),
        "the finding should name the directory the paragraph pinned it to: {}",
        finding.message
    );
}

#[test]
fn nothing_is_reported_when_every_citation_still_resolves() {
    for lang in [
        "paths",
        "commands",
        "flags",
        "symbols",
        "lines",
        "anchors",
        "directory",
    ] {
        let found = findings(lang, "silent");
        assert!(
            found.is_empty(),
            "R1/{lang}/silent cites nothing that is gone, and weeder reported: {found:#?}"
        );
    }
}
