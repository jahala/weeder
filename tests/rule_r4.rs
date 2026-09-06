//! R4, a dependency pin lags the registry.
//!
//! One fixture per manifest kind, each pinning a package the committed snapshot
//! says has moved on. `fire/` is pinned nine minor releases back, past the
//! default allowance of three; `silent/` is pinned two back, inside it. Nothing
//! else about the two differs, so the rule has to be reading the versions.
//!
//! The tests after those are the ones that matter most. A scan reaches no
//! registry: a fetcher on PATH records having been called, and the recording
//! never appears. A refresh is the one command that does reach one, so it is
//! asked what it does when the fetcher is missing, and what it keeps when one
//! registry answers and another cannot be reached.

mod common;

use std::path::Path;

use common::{fixture, install_script, link_git, shell_word, Finding};
use tempfile::TempDir;

/// The manifest kinds R4 reads, the package each fixture pins, and the release
/// the snapshot recorded for it.
const MANIFESTS: &[(&str, &str, &str, &str)] = &[
    ("ts", "package.json", "left-pad", "1.9.0"),
    ("rs", "Cargo.toml", "ordered-float", "1.9.0"),
    ("py", "pyproject.toml", "httpx", "1.9.0"),
    ("go", "go.mod", "example.com/ledger", "1.9.0"),
];

fn findings(lang: &str, case: &str) -> Vec<Finding> {
    let repo = fixture("R4", lang, case);
    let run = repo.weeder(&["scan", "--rules", "R4", "--format", "sarif"]);
    assert_eq!(
        run.code, 0,
        "a scan never blocks, and R4/{lang}/{case} left with {}: {}",
        run.code, run.stderr
    );
    run.findings()
}

#[test]
fn a_pin_further_behind_than_the_threshold_is_reported_for_every_manifest_kind() {
    for (lang, manifest, package, latest) in MANIFESTS {
        let found = findings(lang, "fire");
        assert_eq!(
            found.len(),
            1,
            "R4/{lang}/fire pins one package too far back, and weeder reported: {found:#?}"
        );
        let finding = &found[0];
        assert_eq!(finding.rule, "R4");
        assert_eq!(finding.level, "warning", "a scan finding never blocks");
        assert_eq!(finding.path, *manifest, "the finding names the manifest");
        assert!(
            finding.message.contains(package) && finding.message.contains(latest),
            "R4/{lang}/fire should name `{package}` and the {latest} the snapshot holds: {}",
            finding.message
        );
    }
}

#[test]
fn a_pin_inside_the_threshold_is_left_alone() {
    for (lang, _, _, _) in MANIFESTS {
        let found = findings(lang, "silent");
        assert!(
            found.is_empty(),
            "R4/{lang}/silent is two minor releases back and the default allows three, and weeder reported: {found:#?}"
        );
    }
}

#[test]
fn the_threshold_is_the_repository_s_own() {
    let repo = fixture("R4", "ts", "silent");
    repo.write("weeder.toml", "[thresholds]\ndependency_lag = 1\n");
    let run = repo.weeder(&["scan", "--rules", "R4", "--format", "sarif"]);
    assert_eq!(run.code, 0);
    let found = run.findings();
    assert_eq!(
        found.len(),
        1,
        "a repository that allows one minor release finds this pin too far back, and weeder reported: {found:#?}"
    );
}

#[test]
fn a_scan_reaches_no_registry() {
    // A fetcher that records being asked. If a scan reaches the network at all,
    // it reaches it through here, and the recording is what proves it did.
    let bin = TempDir::new().expect("a directory for the fetcher");
    let recorded = bin.path().join("asked");
    fetcher(bin.path(), &recorded);

    let repo = fixture("R4", "ts", "fire");
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = repo.weeder_with(&["scan", "--format", "sarif"], &[("PATH", &path)]);

    assert_eq!(run.code, 0, "the scan should have run: {}", run.stderr);
    assert!(
        !run.findings().is_empty(),
        "the scan should have judged the pin it was given"
    );
    assert!(
        !recorded.exists(),
        "a scan asked a registry, and the snapshot is a committed file exactly so it never has to"
    );
}

#[test]
fn a_refresh_says_so_when_this_machine_has_no_fetcher() {
    // A PATH with git on it and nothing else. `--refresh-snapshot` is the one
    // thing weeder does over the network, and a machine that cannot do it has to
    // hear that rather than get a snapshot with nothing in it.
    let bin = TempDir::new().expect("a directory for git alone");
    link_git(bin.path());

    let repo = fixture("R4", "ts", "fire");
    let path = bin.path().display().to_string();
    let run = repo.weeder_with(
        &["scan", "--refresh-snapshot", "--format", "sarif"],
        &[("PATH", &path)],
    );

    assert_eq!(
        run.code, 3,
        "a refresh that could not run leaves with 3: {}{}",
        run.stdout, run.stderr
    );
    assert!(
        run.stderr.contains("curl"),
        "the message should name what is missing: {}",
        run.stderr
    );
}

/// A refresh that reached one registry and not the other keeps what the
/// committed snapshot already held for the packages it could not ask about. A
/// snapshot with nothing in it for a package is a rule with nothing to say, so
/// dropping the entry would turn a network failure into a clean bill of health.
#[test]
fn a_refresh_keeps_what_it_could_not_get_a_fresh_answer_about() {
    let bin = TempDir::new().expect("a directory for the fetcher");
    link_git(bin.path());
    install_script(bin.path(), "curl", ONE_REGISTRY_ANSWERS);

    let repo = fixture("R4", "rs", "fire");
    repo.write(
        "package.json",
        "{\n  \"dependencies\": {\n    \"left-pad\": \"1.0.0\"\n  }\n}\n",
    );
    repo.write(
        ".weeder/registry-snapshot.json",
        "{\n  \"cargo\": {\n    \"ordered-float\": \"1.9.0\"\n  },\n  \"npm\": {\n    \"left-pad\": \"1.1.0\"\n  }\n}\n",
    );

    let path = bin.path().display().to_string();
    let run = repo.weeder_with(
        &["scan", "--refresh-snapshot", "--format", "sarif"],
        &[("PATH", &path)],
    );

    assert_eq!(
        run.code, 0,
        "one registry answered, so the refresh happened: {}{}",
        run.stdout, run.stderr
    );
    assert!(
        run.stderr.contains("could not reach crates.io"),
        "the registry weeder could not reach is named: {}",
        run.stderr
    );

    let written = std::fs::read_to_string(repo.root().join(".weeder/registry-snapshot.json"))
        .expect("the snapshot should have been written");
    let snapshot: serde_json::Value = serde_json::from_str(&written).expect("json is json");
    assert_eq!(
        snapshot["npm"]["left-pad"], "2.0.0",
        "the registry that answered is what the snapshot now holds: {written}"
    );
    assert_eq!(
        snapshot["cargo"]["ordered-float"], "1.9.0",
        "the registry weeder could not reach leaves its package as the repository committed it: {written}"
    );
}

/// A fetcher that writes down having been asked and then refuses, so a scan
/// that reached for it is recorded whatever it does with the answer.
fn fetcher(directory: &Path, recorded: &Path) {
    let script = format!(
        "#!/bin/sh\ntouch {}\nexit 1\n",
        shell_word(&recorded.display().to_string())
    );
    install_script(directory, "curl", &script);
}

/// A fetcher one registry answers through and the other refuses: the npm reply
/// is the shape that registry writes, and everything else is a host that does
/// not resolve, which is what a machine with no network says.
const ONE_REGISTRY_ANSWERS: &str = r#"#!/bin/sh
for word in "$@"; do
  case "$word" in
    *registry.npmjs.org*)
      printf '%s' '{"dist-tags":{"latest":"2.0.0"}}'
      exit 0
      ;;
  esac
done
echo "curl: (6) Could not resolve host" >&2
exit 6
"#;
