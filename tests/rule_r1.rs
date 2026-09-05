//! R1, the docs cite something that no longer exists.
//!
//! Four things a document can cite and four authorities that answer for them:
//! the tree for a path, the command's own help for a subcommand and for a flag,
//! and the code for a symbol. Each fixture is a repository whose documentation
//! is true about three of them and wrong about one, so a green run here means
//! weed found that one rather than that it complained about everything.
//!
//! The command fixtures cite `weed` itself, and the binary under test is put on
//! PATH in front of everything else, so the help R1 reads is the help this
//! build prints.

mod common;

use common::{fixture, path_with_weed, Finding};

/// R1's findings on a fixture, with weed on PATH so a cited command resolves.
fn findings(lang: &str, case: &str) -> Vec<Finding> {
    let repo = fixture("R1", lang, case);
    let run = repo.weed_with(
        &["scan", "--rules", "R1", "--format", "sarif"],
        &[("PATH", &path_with_weed())],
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
    assert_eq!(finding.level, "warning", "a scan finding never blocks");
    assert_eq!(finding.path, "README.md");
    finding
}

#[test]
fn a_cited_path_the_tree_no_longer_holds_is_reported() {
    let finding = only("paths");
    assert!(
        finding.message.contains("docs/parser.md"),
        "the finding should name the path that is gone: {}",
        finding.message
    );
}

#[test]
fn a_cited_subcommand_the_command_no_longer_offers_is_reported() {
    let finding = only("commands");
    assert!(
        finding.message.contains("prune"),
        "the finding should name the subcommand that is gone: {}",
        finding.message
    );
}

#[test]
fn a_cited_flag_the_command_no_longer_prints_is_reported() {
    let finding = only("flags");
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
}

#[test]
fn nothing_is_reported_when_every_citation_still_resolves() {
    for lang in ["paths", "commands", "flags", "symbols"] {
        let found = findings(lang, "silent");
        assert!(
            found.is_empty(),
            "R1/{lang}/silent cites nothing that is gone, and weed reported: {found:#?}"
        );
    }
}
