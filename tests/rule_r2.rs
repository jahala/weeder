//! R2, a public symbol has no references.
//!
//! One fixture per language, each a repository with two exports: one the rest of
//! the repository calls and one nothing calls at all. The `silent/` neighbour
//! moves the second export into the file its language treats as an entry point ,
//! a package index, a library root, a test the runner collects, where a caller
//! weeder cannot see is exactly what is expected.

mod common;

use common::{fixture, Finding};

/// The languages R2 reads, and the export each fixture leaves uncalled. The
/// name is written the way its language writes one, which is the point: a rule
/// that only found `formatRecord` would be a TypeScript rule.
const LANGUAGES: &[(&str, &str)] = &[
    ("ts", "formatRecord"),
    ("py", "format_record"),
    ("rs", "format_record"),
    ("go", "FormatRecord"),
];

fn findings(lang: &str, case: &str) -> Vec<Finding> {
    let repo = fixture("R2", lang, case);
    let run = repo.weeder(&["scan", "--rules", "R2", "--format", "sarif"]);
    assert_eq!(
        run.code, 0,
        "a scan never blocks, and R2/{lang}/{case} left with {}: {}",
        run.code, run.stderr
    );
    run.findings()
}

#[test]
fn an_export_nothing_references_is_reported_in_every_language() {
    for (lang, dead) in LANGUAGES {
        let found = findings(lang, "fire");
        assert_eq!(
            found.len(),
            1,
            "R2/{lang}/fire holds one uncalled export, and weeder reported: {found:#?}"
        );
        let finding = &found[0];
        assert_eq!(finding.rule, "R2");
        assert_eq!(finding.level, "warning", "a scan finding never blocks");
        assert!(
            finding.message.contains(dead),
            "R2/{lang}/fire should name `{dead}`: {}",
            finding.message
        );
        assert!(
            finding.line.is_some_and(|line| line >= 1),
            "R2/{lang}/fire should point at the declaration: {finding:#?}"
        );
    }
}

#[test]
fn a_referenced_export_and_an_entry_point_are_left_alone() {
    for (lang, _) in LANGUAGES {
        let found = findings(lang, "silent");
        assert!(
            found.is_empty(),
            "R2/{lang}/silent exports nothing that is unreachable, and weeder reported: {found:#?}"
        );
    }
}
