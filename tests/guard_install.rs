//! `weed guard install` — the hooks written, and whatever was there kept.
//!
//! Every case runs the built binary in a real temp repository and then asks git
//! itself what it now believes, so the proof is git's configuration and the
//! files on disk rather than anything weed says about them.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use common::Repo;

/// The hooks guard installs, in the order it names them.
const HOOKS: [&str; 3] = ["pre-commit", "pre-push", "pre-rebase"];
/// The line a bundle carries to say which binary it calls.
const BINARY_MARKER: &str = "# weed-guard-binary:";
const HOOKS_PATH: [&str; 4] = ["config", "--local", "--get", "core.hooksPath"];

#[test]
fn install_writes_every_hook_and_points_git_at_them() {
    let repo = Repo::init();
    let run = repo.weed(&["guard", "install"]);
    assert_eq!(run.code, 0, "install runs clean\n{}", run.stderr);

    for hook in HOOKS {
        let path = repo.root().join(".githooks").join(hook);
        let script = read(&path);
        assert!(
            script.starts_with("#!/bin/sh"),
            "{hook} is a shell script, not a copy of a binary that goes stale"
        );
        assert!(
            script.contains(&format!("guard {hook}")),
            "{hook} calls its own subcommand:\n{script}"
        );
        assert_eq!(
            resolved(&binary_named(&script)),
            resolved(&common::binary()),
            "{hook} names the binary it was installed from"
        );
        let mode = std::fs::metadata(&path)
            .expect("the hook is on disk")
            .permissions()
            .mode();
        assert!(
            mode & 0o111 != 0,
            "{hook} is executable, or git walks past it without a word"
        );
        assert!(run.stdout.contains(hook), "install says it wrote {hook}");
    }

    assert_eq!(
        repo.git(&HOOKS_PATH).trim(),
        ".githooks",
        "git runs its hooks from where weed wrote them"
    );
}

#[test]
fn install_writes_where_it_is_told_to() {
    let repo = Repo::init();
    let run = repo.weed(&["guard", "install", "--hooks-dir", ".hooks/weed"]);
    assert_eq!(run.code, 0, "{}", run.stderr);

    for hook in HOOKS {
        assert!(
            repo.root().join(".hooks/weed").join(hook).is_file(),
            "{hook} is written where --hooks-dir named"
        );
    }
    assert_eq!(repo.git(&HOOKS_PATH).trim(), ".hooks/weed");
    assert_eq!(
        repo.weed(&["guard", "status"]).code,
        0,
        "status follows the record rather than assuming the default directory"
    );
}

#[test]
fn install_bakes_the_branches_it_was_told_to_protect_into_the_hooks() {
    let repo = Repo::init();
    repo.weed(&["guard", "install", "--protect", "release/*"]);

    for hook in ["pre-push", "pre-rebase"] {
        let script = read(&repo.root().join(".githooks").join(hook));
        assert!(
            script.contains("--protect 'release/*'"),
            "{hook} carries the branches this install named:\n{script}"
        );
    }
    let commit = read(&repo.root().join(".githooks/pre-commit"));
    assert!(
        !commit.contains("--protect"),
        "an index has no branch to protect:\n{commit}"
    );
}

#[test]
fn install_records_the_hooks_path_that_was_there_and_uninstall_puts_it_back() {
    let repo = Repo::init();
    repo.git(&["config", "core.hooksPath", ".their-hooks"]);
    repo.weed(&["guard", "install"]);

    assert_eq!(
        read(&repo.root().join(".git/weed/previous-hooks-path")).trim(),
        ".their-hooks",
        "what the repository had is written down before it is overwritten"
    );

    let run = repo.weed(&["guard", "uninstall"]);
    assert_eq!(run.code, 0, "uninstall runs clean\n{}", run.stderr);
    assert_eq!(
        repo.git(&HOOKS_PATH).trim(),
        ".their-hooks",
        "the setting the repository came with is back"
    );
    for hook in HOOKS {
        assert!(
            !repo.root().join(".githooks").join(hook).exists(),
            "{hook} is gone"
        );
    }
}

#[test]
fn a_second_install_keeps_the_first_record_of_what_was_there() {
    let repo = Repo::init();
    repo.git(&["config", "core.hooksPath", ".their-hooks"]);
    repo.weed(&["guard", "install"]);
    repo.weed(&["guard", "install"]);

    assert_eq!(
        read(&repo.root().join(".git/weed/previous-hooks-path")).trim(),
        ".their-hooks",
        "installing twice must not record guard's own directory as what was there"
    );
    repo.weed(&["guard", "uninstall"]);
    assert_eq!(repo.git(&HOOKS_PATH).trim(), ".their-hooks");
}

#[test]
fn uninstall_unsets_a_hooks_path_this_repository_never_had() {
    let repo = Repo::init();
    repo.weed(&["guard", "install"]);
    repo.weed(&["guard", "uninstall"]);

    let asked = repo.try_git(&HOOKS_PATH);
    assert_ne!(
        asked.code, 0,
        "the setting is gone again, not left pointing at a directory weed emptied: {}",
        asked.stdout
    );
}

#[test]
fn uninstall_removes_only_the_files_it_wrote() {
    let repo = Repo::init();
    repo.weed(&["guard", "install"]);

    let theirs = repo.root().join(".githooks/post-commit");
    let theirs_says = "#!/bin/sh\necho a hook of their own\n";
    std::fs::write(&theirs, theirs_says).expect("their hook should be writable");
    let replaced = repo.root().join(".githooks/pre-push");
    let replaced_says = "#!/bin/sh\necho this one is mine now\n";
    std::fs::write(&replaced, replaced_says).expect("the replacement should be writable");

    let run = repo.weed(&["guard", "uninstall"]);
    assert_eq!(run.code, 0, "{}", run.stderr);

    assert_eq!(
        read(&theirs),
        theirs_says,
        "a hook weed never wrote is not weed's to take away"
    );
    assert_eq!(
        read(&replaced),
        replaced_says,
        "a hook someone replaced is theirs now"
    );
    assert!(
        !repo.root().join(".githooks/pre-commit").exists(),
        "the hooks weed did write are gone"
    );
    assert!(
        run.stdout.contains("pre-push"),
        "uninstall says which file it left behind:\n{}",
        run.stdout
    );
}

#[test]
fn status_reports_every_hook_live_after_install() {
    let repo = Repo::init();
    repo.weed(&["guard", "install"]);

    let run = repo.weed(&["guard", "status"]);
    assert_eq!(
        run.code, 0,
        "nothing is missing\n{}{}",
        run.stdout, run.stderr
    );
    for hook in HOOKS {
        assert!(
            run.stdout
                .lines()
                .any(|line| line.contains(hook) && line.contains("live")),
            "status reports {hook} live:\n{}",
            run.stdout
        );
    }
    assert!(
        run.stdout
            .lines()
            .any(|line| line.contains("core.hooksPath") && line.contains("live")),
        "status reports the setting that makes git look there:\n{}",
        run.stdout
    );
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()))
}

/// The binary a bundle names, read back off its own marker line.
fn binary_named(script: &str) -> PathBuf {
    let named = script
        .lines()
        .find_map(|line| line.strip_prefix(BINARY_MARKER))
        .unwrap_or_else(|| panic!("a hook names the binary it calls:\n{script}"));
    PathBuf::from(named.trim())
}

/// Two spellings of one file compare equal once both have been through here.
fn resolved(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .unwrap_or_else(|error| panic!("{} should be on disk: {error}", path.display()))
}
