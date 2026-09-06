//! D1, a dependency manifest changed.
//!
//! A manifest and its lockfile decide what code the program will be built from,
//! and that is a decision a person makes rather than one an agent slips into a
//! change about something else. So the change is reported wherever it lands,
//! and it stops being a note and starts being a refusal once the run named a
//! scope the manifest is not in.
//!
//! The neighbour is prose about a package: a README naming what to install owns
//! no version and builds nothing.

mod common;

use common::fixture;

/// The manifests and lockfiles each language keeps its dependencies in, as the
/// fire fixture changes them.
const MANIFESTS: [(&str, &[&str]); 4] = [
    ("ts", &["package-lock.json", "package.json"]),
    ("py", &["pyproject.toml", "requirements-test.txt"]),
    ("rs", &["Cargo.lock", "Cargo.toml"]),
    ("go", &["go.mod", "go.sum"]),
];

#[test]
fn d1_warns_on_every_manifest_and_lockfile_in_every_language() {
    for (lang, manifests) in MANIFESTS {
        let repo = fixture("D1", lang, "fire");
        let run = repo.weeder(&["check"]);

        assert_eq!(
            run.code, 0,
            "{lang}: a manifest change is a warning, and a warning does not block\n{}",
            run.stderr
        );
        assert_eq!(
            run.paths(),
            manifests.to_vec(),
            "{lang}: every manifest and lockfile is named, and nothing else is"
        );
        for finding in run.findings() {
            assert_eq!(finding.rule, "D1", "{lang}: the rule is D1");
            assert_eq!(finding.level, "warning", "{lang}: D1 warns by default");
        }
    }
}

#[test]
fn d1_stays_silent_on_a_readme_that_names_a_package() {
    for (lang, _) in MANIFESTS {
        let repo = fixture("D1", lang, "silent");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|line| line == "README.md"),
            "{lang}: the neighbour must be in the diff, or the silence proves nothing"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{lang}: prose naming a package installs nothing"
        );
        assert_eq!(run.code, 0, "{lang}: nothing found, nothing blocked");
    }
}

#[test]
fn d1_blocks_when_the_scope_the_run_was_given_excludes_the_manifest() {
    let repo = fixture("D1", "scope", "fire");
    let run = repo.weeder(&["check"]);

    assert_eq!(
        run.code, 2,
        "a dependency change the run was never scoped for stops it\n{}",
        run.stderr
    );
    let manifest: Vec<common::Finding> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == "D1")
        .collect();
    assert_eq!(manifest.len(), 1, "one manifest, one finding");
    assert_eq!(manifest[0].path, "package.json");
    assert_eq!(
        manifest[0].level, "error",
        "outside the scope the warning becomes a refusal"
    );

    assert!(
        !run.findings()
            .iter()
            .any(|finding| finding.rule == "D1" && finding.path.starts_with("src/")),
        "the file the change was scoped for is not a manifest and carries no D1"
    );
}

#[test]
fn a_manifest_inside_the_scope_is_still_only_a_warning() {
    let repo = fixture("D1", "scope", "fire");
    let run = repo.weeder(&["check", "--scope", "**/*"]);

    let manifest: Vec<common::Finding> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == "D1")
        .collect();
    assert_eq!(manifest.len(), 1, "the same manifest, judged again");
    assert_eq!(
        manifest[0].level, "warning",
        "a scope that allows the manifest leaves D1 where the catalogue put it"
    );
}
