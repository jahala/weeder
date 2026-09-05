//! R4 — a dependency pin lags the registry.
//!
//! One fixture per manifest kind, each pinning a package the committed snapshot
//! says has moved on. `fire/` is pinned nine minor releases back, past the
//! default allowance of three; `silent/` is pinned two back, inside it. Nothing
//! else about the two differs, so the rule has to be reading the versions.
//!
//! The last test is the one that matters most: a scan reaches no registry. It
//! puts a fetcher on PATH that records having been called and then runs a scan,
//! and the recording never appears.

mod common;

use std::path::Path;

use common::{fixture, Finding};
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
    let run = repo.weed(&["scan", "--rules", "R4", "--format", "sarif"]);
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
            "R4/{lang}/fire pins one package too far back, and weed reported: {found:#?}"
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
            "R4/{lang}/silent is two minor releases back and the default allows three, and weed reported: {found:#?}"
        );
    }
}

#[test]
fn the_threshold_is_the_repository_s_own() {
    let repo = fixture("R4", "ts", "silent");
    repo.write("weed.toml", "[thresholds]\ndependency_lag = 1\n");
    let run = repo.weed(&["scan", "--rules", "R4", "--format", "sarif"]);
    assert_eq!(run.code, 0);
    let found = run.findings();
    assert_eq!(
        found.len(),
        1,
        "a repository that allows one minor release finds this pin too far back, and weed reported: {found:#?}"
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
    let run = repo.weed_with(&["scan", "--format", "sarif"], &[("PATH", &path)]);

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
    // thing weed does over the network, and a machine that cannot do it has to
    // hear that rather than get a snapshot with nothing in it.
    let bin = TempDir::new().expect("a directory for git alone");
    link_git(bin.path());

    let repo = fixture("R4", "ts", "fire");
    let path = bin.path().display().to_string();
    let run = repo.weed_with(
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

/// A fetcher that writes down having been asked and then refuses, so a scan
/// that reached for it is recorded whatever it does with the answer.
fn fetcher(directory: &Path, recorded: &Path) {
    let script = format!(
        "#!/bin/sh\ntouch {}\nexit 1\n",
        shell_word(&recorded.display().to_string())
    );
    let path = directory.join("curl");
    std::fs::write(&path, script).expect("the fetcher should be writable");
    make_executable(&path);
}

/// git, where a test has taken everything else off PATH. weed asks git what the
/// tree holds before any rule runs, so a PATH without it is a scan that never
/// starts and proves nothing.
fn link_git(directory: &Path) {
    let found = which("git").expect("git should be on PATH");
    let script = format!(
        "#!/bin/sh\nexec {} \"$@\"\n",
        shell_word(&found.display().to_string())
    );
    let path = directory.join("git");
    std::fs::write(&path, script).expect("the git shim should be writable");
    make_executable(&path);
}

fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("the script should be runnable");
}

/// Where a program on PATH is, asked of the shell that owns the question.
fn which(program: &str) -> Option<std::path::PathBuf> {
    let found = std::process::Command::new("/usr/bin/env")
        .args(["sh", "-c", &format!("command -v {program}")])
        .output()
        .ok()?;
    found
        .status
        .success()
        .then(|| std::path::PathBuf::from(String::from_utf8_lossy(&found.stdout).trim()))
}

/// A path as one word a shell cannot take apart.
fn shell_word(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}
