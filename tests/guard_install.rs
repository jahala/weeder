//! `weeder guard install`, the hooks written, and whatever was there kept.
//!
//! Every case runs the built binary in a real temp repository and then asks git
//! itself what it now believes, so the proof is git's configuration and the
//! files on disk rather than anything weeder says about them.

mod common;

use std::path::{Path, PathBuf};

use common::Repo;

/// The hooks guard installs, in the order it names them, which is the order git
/// runs them: the stage that reads the allowance a person wrote sits between the
/// index and the push.
const HOOKS: [&str; 4] = ["pre-commit", "commit-msg", "pre-push", "pre-rebase"];
/// The stage that reads a `Weeder-allow:` trailer. Without it a guardrail change
/// a person means has no way past the gate at all.
const TRAILER_STAGE: &str = "commit-msg";
/// The line a bundle carries to say which binary it calls.
const BINARY_MARKER: &str = "# weeder-guard-binary:";
const HOOKS_PATH: [&str; 4] = ["config", "--local", "--get", "core.hooksPath"];

#[test]
fn install_writes_every_hook_and_points_git_at_them() {
    let repo = Repo::init();
    let run = repo.weeder(&["guard", "install"]);
    assert_eq!(run.code, 0, "install runs clean\n{}", run.stderr);

    for hook in HOOKS {
        let path = repo.shared_hooks_dir().join(hook);
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
        // What makes a file a hook git runs is the platform's answer, not
        // weeder's. On unix it is the execute bit, and git walks past a hook
        // without one; Windows keeps no such bit and git for Windows asks for
        // none, so there the whole of it is that the file is on disk.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = std::fs::metadata(&path)
                .expect("the hook is on disk")
                .permissions()
                .mode();
            assert!(
                mode & 0o111 != 0,
                "{hook} is executable, or git walks past it without a word"
            );
        }
        #[cfg(windows)]
        assert!(
            path.is_file(),
            "{hook} is on disk, which is the whole of what makes git run it here"
        );
        assert!(run.stdout.contains(hook), "install says it wrote {hook}");
    }

    assert_eq!(
        resolved(&repo.hooks_dir()),
        resolved(&repo.shared_hooks_dir()),
        "git runs its hooks from where weeder wrote them"
    );
}

#[test]
fn install_told_nowhere_writes_beside_the_shared_git_directory_and_names_it_absolutely() {
    let repo = Repo::init();
    let run = repo.weeder(&["guard", "install"]);
    assert_eq!(run.code, 0, "install runs clean\n{}", run.stderr);

    let setting = repo.git(&HOOKS_PATH).trim().to_string();
    assert!(
        Path::new(&setting).is_absolute(),
        "git resolves a relative hooks path against each worktree's own root, so only an \
         absolute one names the same directory in every worktree: {setting}"
    );
    assert_eq!(
        resolved(Path::new(&setting)),
        resolved(&repo.shared_hooks_dir()),
        "the hooks sit beside the git directory every worktree shares"
    );
    for hook in HOOKS {
        assert!(
            repo.shared_hooks_dir().join(hook).is_file(),
            "{hook} is written there"
        );
    }
    assert!(
        repo.git(&["status", "--porcelain", "--untracked-files=all"])
            .trim()
            .is_empty(),
        "nothing is written into the tree a change edits"
    );
}

#[test]
fn one_install_guards_every_worktree_of_the_repository() {
    let repo = Repo::init();
    let elsewhere = tempfile::TempDir::new().expect("a temp directory for the second worktree");
    let second = elsewhere.path().join("second");
    let second_path = second.to_str().expect("a temp path is text");
    repo.git(&["worktree", "add", "-b", "second", second_path]);

    let run = repo.weeder(&["guard", "install"]);
    assert_eq!(run.code, 0, "install runs clean\n{}", run.stderr);

    let status = common::weeder_in(&second, &["guard", "status"]);
    assert_eq!(
        status.code, 0,
        "the second worktree runs the hooks the first one installed\n{}{}",
        status.stdout, status.stderr
    );
    assert!(
        status.stdout.contains("weeder guard is live."),
        "status says so:\n{}",
        status.stdout
    );

    // The proof is git's own: a commit in the second worktree goes through the
    // hook, and the hook refuses what blocks.
    std::fs::create_dir_all(second.join("src")).expect("a directory for the file");
    std::fs::write(
        second.join("src/parser.ts"),
        common::conflicted_parser(None),
    )
    .expect("the file should be writable");
    assert_eq!(common::git_in(&second, &["add", "-A"]).code, 0);
    let refused = common::git_in(&second, &["commit", "-m", "the merge, half finished"]);
    assert_ne!(
        refused.code,
        0,
        "git in the second worktree runs weeder's pre-commit hook: {}",
        refused.output()
    );
    assert!(
        refused.output().contains("G1"),
        "the hook says what it refused:\n{}",
        refused.output()
    );
}

#[test]
fn an_install_made_in_a_second_worktree_guards_the_first() {
    let repo = Repo::init();
    let elsewhere = tempfile::TempDir::new().expect("a temp directory for the second worktree");
    let second = elsewhere.path().join("second");
    let second_path = second.to_str().expect("a temp path is text");
    repo.git(&["worktree", "add", "-b", "second", second_path]);

    let run = common::weeder_in(&second, &["guard", "install"]);
    assert_eq!(run.code, 0, "install runs clean\n{}", run.stderr);

    let status = repo.weeder(&["guard", "status"]);
    assert_eq!(
        status.code, 0,
        "the record and the hooks are the repository's, not the worktree's\n{}{}",
        status.stdout, status.stderr
    );

    let uninstalled = repo.weeder(&["guard", "uninstall"]);
    assert_eq!(uninstalled.code, 0, "{}", uninstalled.stderr);
    assert_ne!(
        repo.try_git(&HOOKS_PATH).code,
        0,
        "uninstall from either worktree takes the setting away again"
    );
    for hook in HOOKS {
        assert!(
            !repo.shared_hooks_dir().join(hook).exists(),
            "{hook} is gone"
        );
    }
}

#[test]
fn an_install_already_at_githooks_keeps_its_directory() {
    let repo = Repo::init();
    let first = repo.weeder(&["guard", "install", "--hooks-dir", ".githooks"]);
    assert_eq!(first.code, 0, "{}", first.stderr);

    let again = repo.weeder(&["guard", "install"]);
    assert_eq!(again.code, 0, "{}", again.stderr);
    assert_eq!(
        repo.git(&HOOKS_PATH).trim(),
        ".githooks",
        "a repository that chose to carry its hooks keeps carrying them"
    );
    for hook in HOOKS {
        assert!(
            repo.root().join(".githooks").join(hook).is_file(),
            "{hook} is written where it was before"
        );
    }
    assert!(
        !repo.shared_hooks_dir().exists(),
        "nothing is written to a second directory git never runs"
    );
    assert_eq!(repo.weeder(&["guard", "status"]).code, 0);

    let uninstalled = repo.weeder(&["guard", "uninstall"]);
    assert_eq!(uninstalled.code, 0, "{}", uninstalled.stderr);
    assert!(!repo.root().join(".githooks/pre-commit").exists());
}

#[test]
fn hooks_written_into_the_tree_name_weeder_by_path_and_no_path_of_this_machine() {
    let repo = Repo::init();
    let run = repo.weeder(&["guard", "install", "--hooks-dir", ".githooks"]);
    assert_eq!(run.code, 0, "{}", run.stderr);

    let installer = common::binary().display().to_string();
    let home = std::env::var("HOME").unwrap_or_default();
    for hook in HOOKS {
        let script = read(&repo.root().join(".githooks").join(hook));
        assert!(
            !script.contains(&installer),
            "{hook} is a file a commit carries, so it names no path of the machine that \
             installed it:\n{script}"
        );
        assert!(
            home.is_empty() || !script.contains(&home),
            "{hook} names nothing under the installer's home:\n{script}"
        );
        assert_eq!(
            binary_named(&script),
            PathBuf::from("weeder"),
            "{hook} runs the weeder on PATH"
        );
    }

    let status = repo.weeder_with(
        &["guard", "status"],
        &[("PATH", &common::path_with_weeder())],
    );
    assert_eq!(
        status.code, 0,
        "weeder is on PATH, so every hook is live\n{}{}",
        status.stdout, status.stderr
    );
}

/// What a hook that names weeder by PATH does on a machine whose PATH has none:
/// git runs it, it says so, and the commit stops. The PATH here holds git and
/// the shell alone, which takes a stand-in script for git, and Windows starts
/// no script as a program, so this half is unix's; the half above is not.
#[cfg(unix)]
#[test]
fn a_hook_that_names_weeder_by_path_stops_the_commit_where_path_has_none() {
    let repo = Repo::init();
    let run = repo.weeder(&["guard", "install", "--hooks-dir", ".githooks"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    repo.commit("weeder guard installed");

    let bare = tempfile::TempDir::new().expect("a directory for a PATH without weeder");
    common::link_git(bare.path());
    let without = format!("{}:/bin", bare.path().display());
    let with = common::path_with_weeder();

    let status = repo.weeder_with(&["guard", "status"], &[("PATH", &without)]);
    assert_eq!(
        status.code, 2,
        "no weeder on PATH is a miss\n{}{}",
        status.stdout, status.stderr
    );
    assert!(
        status.stdout.contains("PATH"),
        "status says it looked on PATH:\n{}",
        status.stdout
    );

    repo.write(
        "src/parser.ts",
        "export const parse = (input: string) => input;\n",
    );
    repo.stage_all();
    let refused = repo.try_git_with(&["commit", "-m", "the parser"], &[("PATH", &without)]);
    assert_ne!(
        refused.code,
        0,
        "a hook that cannot find weeder stops the commit rather than waving it through: {}",
        refused.output()
    );
    assert!(
        refused.output().contains("PATH"),
        "the hook says where it looked:\n{}",
        refused.output()
    );

    let landed = repo.try_git_with(&["commit", "-m", "the parser"], &[("PATH", &with)]);
    assert_eq!(
        landed.code,
        0,
        "with weeder on PATH the same hook runs it: {}",
        landed.output()
    );
}

#[test]
fn install_writes_where_it_is_told_to() {
    let repo = Repo::init();
    let run = repo.weeder(&["guard", "install", "--hooks-dir", ".hooks/weeder"]);
    assert_eq!(run.code, 0, "{}", run.stderr);

    for hook in HOOKS {
        assert!(
            repo.root().join(".hooks/weeder").join(hook).is_file(),
            "{hook} is written where --hooks-dir named"
        );
    }
    assert_eq!(repo.git(&HOOKS_PATH).trim(), ".hooks/weeder");
    assert_eq!(
        repo.weeder(&["guard", "status"]).code,
        0,
        "status follows the record rather than assuming the default directory"
    );
}

#[test]
fn install_bakes_the_branches_it_was_told_to_protect_into_the_hooks() {
    let repo = Repo::init();
    repo.weeder(&["guard", "install", "--protect", "release/*"]);

    for hook in ["pre-push", "pre-rebase"] {
        let script = read(&repo.hooks_dir().join(hook));
        assert!(
            script.contains("--protect 'release/*'"),
            "{hook} carries the branches this install named:\n{script}"
        );
    }
    for hook in ["pre-commit", TRAILER_STAGE] {
        let script = read(&repo.hooks_dir().join(hook));
        assert!(
            !script.contains("--protect"),
            "an index has no branch to protect:\n{script}"
        );
    }
}

#[test]
fn the_commit_msg_hook_is_handed_the_file_git_writes_the_message_in() {
    let repo = Repo::init();
    repo.weeder(&["guard", "install"]);

    let script = read(&repo.hooks_dir().join(TRAILER_STAGE));
    assert!(
        script.contains(&format!("guard {TRAILER_STAGE} -- \"$@\"")),
        "the hook passes git's own argument through, and without it there is no message to read \
         an allowance from:\n{script}"
    );
}

#[test]
fn status_names_a_missing_commit_msg_and_refuses_to_call_the_rest_the_law() {
    let repo = Repo::init();
    repo.weeder(&["guard", "install"]);
    std::fs::remove_file(repo.hooks_dir().join(TRAILER_STAGE)).expect("the hook is on disk");

    let run = repo.weeder(&["guard", "status"]);

    assert_eq!(
        run.code, 2,
        "the stage that reads an allowance is gone, and that is a miss\n{}{}",
        run.stdout, run.stderr
    );
    assert!(
        run.stdout
            .lines()
            .any(|line| line.contains(TRAILER_STAGE) && line.contains("missing")),
        "status names the hook and the miss:\n{}",
        run.stdout
    );
}

#[test]
fn the_commit_that_first_carries_the_four_goes_through_the_hooks_it_installs() {
    // A repository that carries its hooks in the tree asks for that by name;
    // told nowhere, install writes outside the tree and there is nothing to commit.
    let repo = Repo::init();
    let run = repo.weeder(&["guard", "install", "--hooks-dir", ".githooks"]);
    assert_eq!(run.code, 0, "install runs clean\n{}", run.stderr);

    repo.stage_all();
    let adopted = repo.try_git(&["commit", "-m", "weeder guard installed"]);

    assert_eq!(
        adopted.code,
        0,
        "a hook is a guardrail path, and C1 knows weeder's own bundle byte for byte, so adopting \
         weeder is an ordinary commit rather than one that needs an allowance: {}",
        adopted.output()
    );
    let carried = repo.git(&["show", "--format=", "--name-only", "HEAD"]);
    for hook in HOOKS {
        assert!(
            carried
                .lines()
                .any(|line| line == format!(".githooks/{hook}")),
            "the commit carries {hook}:\n{carried}"
        );
    }
}

#[test]
fn install_records_the_hooks_path_that_was_there_and_uninstall_puts_it_back() {
    let repo = Repo::init();
    repo.git(&["config", "core.hooksPath", ".their-hooks"]);
    let run = repo.weeder(&["guard", "install", "--replace"]);
    assert_eq!(
        run.code, 0,
        "asked to replace, install runs\n{}",
        run.stderr
    );

    assert_eq!(
        read(&repo.root().join(".git/weeder/previous-hooks-path")).trim(),
        ".their-hooks",
        "what the repository had is written down before it is overwritten"
    );

    let run = repo.weeder(&["guard", "uninstall"]);
    assert_eq!(run.code, 0, "uninstall runs clean\n{}", run.stderr);
    assert_eq!(
        repo.git(&HOOKS_PATH).trim(),
        ".their-hooks",
        "the setting the repository came with is back"
    );
    for hook in HOOKS {
        assert!(
            !repo.shared_hooks_dir().join(hook).exists(),
            "{hook} is gone"
        );
    }
}

#[test]
fn a_second_install_keeps_the_first_record_of_what_was_there() {
    let repo = Repo::init();
    repo.git(&["config", "core.hooksPath", ".their-hooks"]);
    repo.weeder(&["guard", "install", "--replace"]);
    let again = repo.weeder(&["guard", "install"]);
    assert_eq!(
        again.code, 0,
        "git already runs weeder's own hooks, so installing again replaces nobody's\n{}",
        again.stderr
    );

    assert_eq!(
        read(&repo.root().join(".git/weeder/previous-hooks-path")).trim(),
        ".their-hooks",
        "installing twice must not record guard's own directory as what was there"
    );
    repo.weeder(&["guard", "uninstall"]);
    assert_eq!(repo.git(&HOOKS_PATH).trim(), ".their-hooks");
}

#[test]
fn install_refuses_to_move_git_away_from_a_hooks_path_another_tool_set() {
    let repo = Repo::init();
    repo.write(".husky/pre-commit", "#!/bin/sh\nnpm test\n");
    repo.git(&["config", "core.hooksPath", ".husky"]);

    let run = repo.weeder(&["guard", "install"]);
    assert_eq!(
        run.code, 3,
        "installing would stop the hooks in .husky running, so it is refused\n{}{}",
        run.stdout, run.stderr
    );
    assert!(
        run.stderr.contains(".husky"),
        "the refusal names the directory git runs hooks from now: {}",
        run.stderr
    );
    assert!(
        run.stderr.contains("--replace") && run.stderr.contains("weeder guard uninstall"),
        "the refusal says how to go on, and how to come back: {}",
        run.stderr
    );
    assert_eq!(
        repo.git(&HOOKS_PATH).trim(),
        ".husky",
        "git still runs the hooks it ran before"
    );
    assert!(
        !repo.shared_hooks_dir().exists(),
        "a refused install writes nothing"
    );

    let replaced = repo.weeder(&["guard", "install", "--replace"]);
    assert_eq!(replaced.code, 0, "{}", replaced.stderr);
    assert_eq!(
        resolved(&repo.hooks_dir()),
        resolved(&repo.shared_hooks_dir())
    );
    repo.weeder(&["guard", "uninstall"]);
    assert_eq!(
        repo.git(&HOOKS_PATH).trim(),
        ".husky",
        "uninstall hands git back to the hooks it replaced"
    );
}

#[test]
fn install_into_the_hooks_directory_git_already_runs_silences_nothing() {
    let repo = Repo::init();
    let own = git_hooks(&repo);
    let theirs = own.join("post-merge");
    let theirs_says = "#!/bin/sh\necho a hook of their own\n";
    std::fs::write(&theirs, theirs_says).expect("their hook should be writable");
    common::make_runnable(&theirs);

    let named = own.display().to_string();
    let run = repo.weeder(&["guard", "install", "--hooks-dir", &named]);
    assert_eq!(
        run.code, 0,
        "git already runs this directory, so writing beside their hook stops nothing\n{}{}",
        run.stdout, run.stderr
    );
    assert_eq!(read(&theirs), theirs_says, "their hook is as it was");
    for hook in HOOKS {
        assert!(own.join(hook).is_file(), "{hook} is written beside it");
    }
    let status = repo.weeder(&["guard", "status"]);
    assert_eq!(
        status.code, 0,
        "the install is live\n{}{}",
        status.stdout, status.stderr
    );
}

#[test]
fn install_never_writes_over_a_hook_of_the_same_name_that_does_not_call_weeder() {
    let repo = Repo::init();
    let own = git_hooks(&repo);
    let theirs = own.join("pre-push");
    let theirs_says = "#!/bin/sh\n./scripts/guard-push.sh\n";
    std::fs::write(&theirs, theirs_says).expect("their hook should be writable");
    common::make_runnable(&theirs);
    let named = own.display().to_string();

    let refused = repo.weeder(&["guard", "install", "--hooks-dir", &named]);
    assert_eq!(
        refused.code, 3,
        "installing would overwrite their pre-push, so it is refused\n{}{}",
        refused.stdout, refused.stderr
    );
    assert!(
        refused.stderr.contains("pre-push") && refused.stderr.contains("weeder guard pre-push"),
        "the refusal names the hook and the call to add to it: {}",
        refused.stderr
    );
    assert_eq!(read(&theirs), theirs_says, "their hook is as it was");
    assert!(
        !own.join("pre-commit").exists(),
        "a refused install writes nothing"
    );
    assert_ne!(
        repo.try_git(&HOOKS_PATH).code,
        0,
        "the setting was never written"
    );

    // The person does what the refusal says, and the hook is the stage now.
    let calling = format!(
        "#!/bin/sh\n./scripts/guard-push.sh\nexec {} guard pre-push -- \"$@\"\n",
        common::shell_word(&common::binary().display().to_string())
    );
    std::fs::write(&theirs, &calling).expect("their hook should be writable");
    let run = repo.weeder(&["guard", "install", "--hooks-dir", &named]);
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        read(&theirs),
        calling,
        "a hook that calls weeder is left as it is"
    );
    let status = repo.weeder(&["guard", "status"]);
    assert_eq!(
        status.code, 0,
        "their pre-push is the stage, and the rest are weeder's\n{}{}",
        status.stdout, status.stderr
    );
}

#[test]
fn install_refuses_to_silence_the_hooks_git_keeps_of_its_own() {
    let repo = Repo::init();
    let theirs = repo.root().join(".git/hooks/pre-push");
    std::fs::write(&theirs, "#!/bin/sh\ngit lfs pre-push \"$@\"\n")
        .expect("their hook should be writable");
    common::make_runnable(&theirs);

    let run = repo.weeder(&["guard", "install"]);
    assert_eq!(
        run.code, 3,
        "pointing core.hooksPath anywhere stops .git/hooks running, so it is refused\n{}{}",
        run.stdout, run.stderr
    );
    assert!(
        run.stderr.contains("pre-push") && run.stderr.contains("--replace"),
        "the refusal names the hook that would stop running and how to go on: {}",
        run.stderr
    );
    assert_ne!(
        repo.try_git(&HOOKS_PATH).code,
        0,
        "the setting was never written"
    );
}

#[test]
fn uninstall_unsets_a_hooks_path_this_repository_never_had() {
    let repo = Repo::init();
    repo.weeder(&["guard", "install"]);
    repo.weeder(&["guard", "uninstall"]);

    let asked = repo.try_git(&HOOKS_PATH);
    assert_ne!(
        asked.code, 0,
        "the setting is gone again, not left pointing at a directory weeder emptied: {}",
        asked.stdout
    );
}

#[test]
fn uninstall_removes_only_the_files_it_wrote() {
    let repo = Repo::init();
    repo.weeder(&["guard", "install"]);

    let theirs = repo.hooks_dir().join("post-commit");
    let theirs_says = "#!/bin/sh\necho a hook of their own\n";
    std::fs::write(&theirs, theirs_says).expect("their hook should be writable");
    let replaced = repo.hooks_dir().join("pre-push");
    let replaced_says = "#!/bin/sh\necho this one is mine now\n";
    std::fs::write(&replaced, replaced_says).expect("the replacement should be writable");

    let run = repo.weeder(&["guard", "uninstall"]);
    assert_eq!(run.code, 0, "{}", run.stderr);

    assert_eq!(
        read(&theirs),
        theirs_says,
        "a hook weeder never wrote is not weeder's to take away"
    );
    assert_eq!(
        read(&replaced),
        replaced_says,
        "a hook someone replaced is theirs now"
    );
    assert!(
        !repo.shared_hooks_dir().join("pre-commit").exists(),
        "the hooks weeder did write are gone"
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
    repo.weeder(&["guard", "install"]);

    let run = repo.weeder(&["guard", "status"]);
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

/// The hooks directory git keeps of its own, in the git directory every
/// worktree shares, made if git did not.
fn git_hooks(repo: &Repo) -> PathBuf {
    let common = repo.git(&["rev-parse", "--path-format=absolute", "--git-common-dir"]);
    let hooks = PathBuf::from(common.trim()).join("hooks");
    std::fs::create_dir_all(&hooks).expect("git's own hooks directory");
    hooks
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
