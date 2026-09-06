//! `weed guard status`, every way a hook stops being the law, named.
//!
//! A gate that has quietly stopped running is worse than no gate, so each case
//! breaks the installation in one way and asks status what it can see.

mod common;

use std::os::unix::fs::PermissionsExt;

use common::Repo;

#[test]
fn status_says_so_when_guard_has_installed_nothing() {
    let repo = Repo::init();

    let run = repo.weed(&["guard", "status"]);

    assert_eq!(run.code, 2, "nothing installed is a miss\n{}", run.stderr);
    assert!(
        run.stdout.contains("weed guard install"),
        "status says what to do about it:\n{}",
        run.stdout
    );
}

#[test]
fn status_names_a_hook_that_is_missing() {
    let repo = installed();
    std::fs::remove_file(repo.root().join(".githooks/pre-push")).expect("the hook is on disk");

    let run = repo.weed(&["guard", "status"]);

    assert_eq!(run.code, 2, "a missing hook is a miss\n{}", run.stderr);
    assert!(
        names(&run.stdout, "pre-push", "missing"),
        "status names the hook and the miss:\n{}",
        run.stdout
    );
    assert!(
        names(&run.stdout, "pre-commit", "live"),
        "the hooks that are still there are still reported:\n{}",
        run.stdout
    );
}

#[test]
fn status_names_a_hook_git_cannot_run() {
    let repo = installed();
    let hook = repo.root().join(".githooks/pre-commit");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o644))
        .expect("the hook's mode is the test's to change");

    let run = repo.weed(&["guard", "status"]);

    assert_eq!(run.code, 2, "a hook git skips is a miss\n{}", run.stderr);
    assert!(
        names(&run.stdout, "pre-commit", "not executable"),
        "status names the hook and the miss:\n{}",
        run.stdout
    );
}

#[test]
fn status_names_a_hooks_path_that_points_elsewhere() {
    let repo = installed();
    repo.git(&["config", "core.hooksPath", ".their-hooks"]);

    let run = repo.weed(&["guard", "status"]);

    assert_eq!(
        run.code, 2,
        "git looking somewhere else is a miss\n{}",
        run.stderr
    );
    assert!(
        names(&run.stdout, "core.hooksPath", ".their-hooks"),
        "status names the setting and where it now points:\n{}",
        run.stdout
    );
}

#[test]
fn status_names_a_hooks_path_that_is_set_no_more() {
    let repo = installed();
    repo.git(&["config", "--unset", "core.hooksPath"]);

    let run = repo.weed(&["guard", "status"]);

    assert_eq!(
        run.code, 2,
        "git running its own hooks again is a miss\n{}",
        run.stderr
    );
    assert!(
        run.stdout.contains("core.hooksPath"),
        "status names the setting:\n{}",
        run.stdout
    );
}

#[test]
fn status_names_a_hook_whose_binary_is_gone() {
    let repo = Repo::init();
    let elsewhere = tempfile::TempDir::new().expect("a temp directory for the copy");
    let copy = elsewhere.path().join("weed");
    std::fs::copy(common::binary(), &copy).expect("weed should copy");
    std::fs::set_permissions(&copy, std::fs::Permissions::from_mode(0o755))
        .expect("the copy should be runnable");

    // Installed by the copy, so the hooks name the copy, and then the copy goes
    // the way a binary goes when a checkout moves or a release is cleaned up.
    let installed = common::command_in(&copy, repo.root(), &["guard", "install"])
        .output()
        .expect("the copy should run");
    assert!(
        installed.status.success(),
        "the copy installs: {}",
        String::from_utf8_lossy(&installed.stderr)
    );
    std::fs::remove_file(&copy).expect("the copy is the test's to take away");

    let run = repo.weed(&["guard", "status"]);

    assert_eq!(
        run.code, 2,
        "a hook naming a binary that is gone is a miss\n{}",
        run.stderr
    );
    assert!(
        run.stdout.contains(&copy.display().to_string()),
        "status names the binary the hook is calling:\n{}",
        run.stdout
    );
}

#[test]
fn status_is_clean_when_nothing_is_broken() {
    let repo = installed();

    let run = repo.weed(&["guard", "status"]);

    assert_eq!(run.code, 0, "an untouched install is live\n{}", run.stderr);
}

fn installed() -> Repo {
    let repo = Repo::init();
    let run = repo.weed(&["guard", "install"]);
    assert_eq!(run.code, 0, "install runs clean\n{}", run.stderr);
    repo
}

/// Whether one line of the report carries both the thing and what is wrong with
/// it. A report that names the miss somewhere else on the page is a report that
/// makes a person guess.
fn names(report: &str, thing: &str, miss: &str) -> bool {
    report
        .lines()
        .any(|line| line.contains(thing) && line.contains(miss))
}
