//! D1, a dependency manifest changed.
//!
//! A manifest and its lockfile decide what code the program will be built from,
//! and that is a decision a person makes rather than one an agent slips into a
//! change about something else. So the change is reported wherever it lands,
//! and it stops being a note and starts being a refusal once the run named a
//! scope the manifest is not in.
//!
//! The neighbour is prose about a package: a README naming what to install owns
//! no version and builds nothing. The nearer neighbour is a release: a change
//! that moves only the version the package calls itself moves no dependency,
//! and D1 says nothing about it. Which `version` is the package's own is read
//! from where it sits in the file, because the same line of text pins a
//! dependency one table further down.

mod common;

use common::{fixture, Repo};

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

/// The manifests and lockfiles a release moves the package's own version in,
/// and the line of each where the fire fixture moves a dependency's version
/// beside it. Go has no such line: `go.mod` names the module and what it
/// requires, and a release of a Go module is a tag rather than a line in a
/// file, so every change to `go.mod` is a dependency change and the go fire
/// fixture above holds that.
const OWN_VERSIONS: [(&str, &[(&str, u64)]); 3] = [
    ("ts", &[("package-lock.json", 12), ("package.json", 5)]),
    ("py", &[("pyproject.toml", 4)]),
    ("rs", &[("Cargo.lock", 12), ("Cargo.toml", 7)]),
];

#[test]
fn d1_stays_silent_when_only_the_package_s_own_version_moved() {
    for (lang, manifests) in OWN_VERSIONS {
        let repo = fixture("D1", lang, "silent-own-version");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        for (manifest, _) in manifests {
            assert!(
                changed.lines().any(|line| line == *manifest),
                "{lang}: {manifest} must be in the diff, or the silence proves nothing"
            );
        }

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{lang}: a release that moves only its own version moves no dependency"
        );
        assert_eq!(run.code, 0, "{lang}: nothing found, nothing blocked");
    }
}

#[test]
fn d1_still_warns_when_a_dependency_s_version_moved_beside_the_own_one() {
    for (lang, manifests) in OWN_VERSIONS {
        let repo = fixture("D1", lang, "fire-dependency-version");
        let run = repo.weeder(&["check"]);

        assert_eq!(run.code, 0, "{lang}: still a warning\n{}", run.stderr);
        let found: Vec<(String, Option<u64>)> = run
            .findings()
            .into_iter()
            .map(|finding| {
                assert_eq!(finding.rule, "D1", "{lang}: the rule is D1");
                assert_eq!(finding.level, "warning", "{lang}: D1 warns by default");
                (finding.path, finding.line)
            })
            .collect();
        let expected: Vec<(String, Option<u64>)> = manifests
            .iter()
            .map(|(path, line)| (path.to_string(), Some(*line)))
            .collect();
        assert_eq!(
            found, expected,
            "{lang}: every manifest that moved a dependency is named once, at the \
             dependency's line rather than at the release's"
        );
    }
}

/// A repository whose one manifest is `before` at HEAD and `after` in the tree.
fn a_manifest_changed(path: &str, before: &str, after: &str) -> Repo {
    let repo = Repo::init();
    repo.write(path, before);
    repo.commit("the state the change starts from");
    repo.write(path, after);
    repo.stage_all();
    repo
}

#[test]
fn a_workspace_s_own_version_is_its_own_too() {
    let before = "[workspace]\nmembers = [\"demo\"]\n\n[workspace.package]\nversion = \"1.4.0\"\n\n[workspace.dependencies.left-pad]\nversion = \"1.3.0\"\n";

    let repo = a_manifest_changed("Cargo.toml", before, &before.replace("1.4.0", "1.5.0"));
    assert_eq!(
        repo.weeder(&["check"]).findings(),
        Vec::new(),
        "the version every member inherits is the workspace's own"
    );

    let repo = a_manifest_changed("Cargo.toml", before, &before.replace("1.3.0", "1.3.1"));
    let found = repo.weeder(&["check"]).findings();
    assert_eq!(found.len(), 1, "a workspace dependency's version is a pin");
    assert_eq!((found[0].rule.as_str(), found[0].line), ("D1", Some(8)));
}

#[test]
fn a_poetry_project_s_own_version_is_its_own_too() {
    let before = "[tool.poetry]\nname = \"demo\"\nversion = \"1.4.0\"\n\n[tool.poetry.dependencies.tomli]\nversion = \"2.0.1\"\n";

    let repo = a_manifest_changed("pyproject.toml", before, &before.replace("1.4.0", "1.5.0"));
    assert_eq!(
        repo.weeder(&["check"]).findings(),
        Vec::new(),
        "poetry keeps the project's own version under [tool.poetry]"
    );

    let repo = a_manifest_changed("pyproject.toml", before, &before.replace("2.0.1", "2.0.2"));
    let found = repo.weeder(&["check"]).findings();
    assert_eq!(found.len(), 1, "a poetry dependency's version is a pin");
    assert_eq!((found[0].rule.as_str(), found[0].line), ("D1", Some(6)));
}

#[test]
fn a_line_that_holds_the_own_version_and_a_dependency_is_read_whole() {
    // One line can carry the own version and a dependency at once. The own
    // version moving must not carry the dependency through with it.
    let before = "{\"name\": \"demo\", \"version\": \"1.4.0\", \"dependencies\": {\"left-pad\": \"1.3.0\"}}\n";

    let repo = a_manifest_changed(
        "package.json",
        before,
        &before.replace("1.4.0", "1.5.0").replace("1.3.0", "1.3.1"),
    );
    let found = repo.weeder(&["check"]).findings();
    assert_eq!(
        found.len(),
        1,
        "the dependency on the same line still moved"
    );

    let repo = a_manifest_changed("package.json", before, &before.replace("1.4.0", "1.5.0"));
    assert_eq!(
        repo.weeder(&["check"]).findings(),
        Vec::new(),
        "everything else on the line is as it was"
    );
}
