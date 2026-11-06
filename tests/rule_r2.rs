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

/// The files `[entrypoints] loaded` names are loaded by another program from
/// the path it is given, so nothing in the repository imports what they export,
/// by design. Each fixture's own `weeder.toml` names its files, and the same
/// tree without that list reports each of them, which is what the list is for.
#[test]
fn a_file_another_program_loads_by_path_is_an_entry_point_when_the_config_names_it() {
    let loaded: &[(&str, &[&str])] = &[
        ("ts", &["loops/first/loop.ts", "runner.config.ts"]),
        ("py", &["loops/first/loop.py"]),
        ("rs", &["loops/first/loop.rs"]),
        ("go", &["loops/first/loop.go"]),
    ];
    for (lang, paths) in loaded {
        let repo = fixture("R2", lang, "loaded");
        let run = repo.weeder(&["scan", "--rules", "R2", "--format", "sarif"]);
        assert_eq!(
            run.code, 0,
            "R2/{lang}/loaded writes `[entrypoints] loaded`, a key weeder reads: {}",
            run.stderr
        );
        let found = run.findings();
        assert!(
            found.is_empty(),
            "R2/{lang}/loaded names every file it exports from as loaded by path, and weeder reported: {found:#?}"
        );

        repo.write("weeder.toml", "");
        let run = repo.weeder(&["scan", "--rules", "R2", "--format", "sarif"]);
        assert_eq!(run.code, 0, "a scan never blocks: {}", run.stderr);
        let reported: Vec<String> = run
            .findings()
            .into_iter()
            .map(|finding| finding.path)
            .collect();
        assert_eq!(
            reported,
            paths
                .iter()
                .map(|path| path.to_string())
                .collect::<Vec<String>>(),
            "R2/{lang}/loaded without the list reports every file it would have named"
        );
    }
}

/// An export written under another name is imported by that name, so that is
/// the name R2 looks for. The fixture exports two functions under aliases and
/// the rest of the repository imports one of them: the other is the finding,
/// named the way an importer would have to write it.
///
/// Only TypeScript and JavaScript export a name under another one. Python's
/// `from a import b as c` and Rust's `pub use a::b as c` are imports, which R2
/// does not judge, and Go exports a name by how it is spelled.
#[test]
fn an_aliased_export_is_looked_up_by_the_name_it_is_exported_as() {
    let found = findings("ts", "aliased");
    assert_eq!(
        found.len(),
        1,
        "R2/ts/aliased exports one alias nothing imports, and weeder reported: {found:#?}"
    );
    assert!(
        found[0].message.starts_with("`parseNode` is exported"),
        "the finding names the alias an importer would write, not the local name: {}",
        found[0].message
    );
    assert_eq!(found[0].path, "src/plan.ts");
}
