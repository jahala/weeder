//! The trailer stage recognised by what a hook invokes, not by weeder's marker.
//!
//! The umbrella's stem renders hooks from a manifest and carries no bed's
//! private text, so the commit-msg hook it writes calls the weeder binary by a
//! path of the stem's own and has no marker line. That hook is the stage: it
//! runs `weeder guard commit-msg` on the message a person wrote, which is the
//! whole of what pre-commit defers to. A file that only mentions the subcommand
//! in a comment, or that calls another one, runs nothing of the sort, and
//! pre-commit refuses on its own rather than handing the verdict to it.
//!
//! Nothing here is mocked: the hook is a real file at `core.hooksPath`, git runs
//! it, and the commit either exists afterwards or does not.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use common::Repo;

/// The rule a guardrail change fires, and the one a person allows here.
const RULE: &str = "C1";
/// The guardrail change being made: a hook of the repository's own, beside the
/// four weeder wrote.
const GUARDRAIL: &str = ".githooks/post-commit";
const PLANTED: &str = "#!/bin/sh\n# the repository's own hook, planted beside weeder's\nexit 0\n";
/// The reason a person gives for it.
const REASON: &str = "the stem plants its own hook, and the owner asked for it";
/// Where the stem keeps the binary it fetched, which is not where weeder's own
/// install would have found one.
const STEM_BINARY: &str = ".plotplot/bin/weeder";
/// The hook the stem writes, at the path git already runs a commit-msg hook from.
const STAGE: &str = ".githooks/commit-msg";

/// The hook the stem renders: git's own arguments handed to `guard commit-msg`
/// through a binary the stem fetched, and not a marker line of weeder's anywhere.
fn the_stage() -> String {
    stem_shaped(&format!(
        "exec \"$root/{STEM_BINARY}\" guard commit-msg \"$@\""
    ))
}

/// The same words, in a comment. A comment invokes nothing.
fn only_a_comment() -> String {
    format!(
        "#!/bin/sh\n\
         # the stage would be: exec \"$root/{STEM_BINARY}\" guard commit-msg \"$@\"\n\
         exit 0\n"
    )
}

/// A hook that really does call weeder, at the other subcommand. It judges an
/// index and reads no message, so it is not the stage either.
fn another_subcommand() -> String {
    stem_shaped(&format!("exec \"$root/{STEM_BINARY}\" guard pre-commit"))
}

fn stem_shaped(invocation: &str) -> String {
    format!("#!/bin/sh\nset -eu\nroot=$(git rev-parse --show-toplevel)\n{invocation}\n")
}

#[test]
fn pre_commit_defers_to_the_stage_another_planter_wrote() {
    let repo = planted(&the_stage());
    let before = repo.head();
    staged(&repo);

    let allowed = repo.try_git(&["commit", "-m", &allowing(RULE, REASON)]);

    assert_eq!(
        allowed.code,
        0,
        "the stage is installed, by another planter and by another path, so the trailer is read: {}",
        allowed.output()
    );
    assert_ne!(repo.head(), before, "the commit is really there");
    assert_eq!(
        repo.git(&["show", "--format=", "--name-only", "HEAD"])
            .trim(),
        GUARDRAIL,
        "the commit carries the guardrail change the trailer allowed"
    );
}

#[test]
fn the_stage_another_planter_wrote_still_refuses_a_message_that_allows_nothing() {
    let repo = planted(&the_stage());
    let before = repo.head();
    staged(&repo);

    let refused = repo.try_git(&["commit", "-m", "plant the repository's own hook"]);

    assert_ne!(
        refused.code,
        0,
        "deferring to the stage is not waving the change through: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(
        said.contains(RULE),
        "the stage names the rule that blocks:\n{said}"
    );
    assert_eq!(repo.head(), before, "no commit was made");
}

#[test]
fn a_commented_invocation_is_not_the_stage() {
    let repo = planted(&only_a_comment());
    let before = repo.head();
    staged(&repo);

    let refused = repo.try_git(&["commit", "-m", &allowing(RULE, REASON)]);

    assert_ne!(
        refused.code,
        0,
        "a file that mentions the stage in a comment runs none of it: {}",
        refused.output()
    );
    assert!(
        refused.output().contains("weeder guard refused"),
        "pre-commit refuses on its own rather than deferring to a comment:\n{}",
        refused.output()
    );
    assert_eq!(repo.head(), before, "no commit was made");
}

#[test]
fn a_hook_that_invokes_another_subcommand_is_not_the_stage() {
    let repo = planted(&another_subcommand());
    let before = repo.head();
    staged(&repo);

    let refused = repo.try_git(&["commit", "-m", &allowing(RULE, REASON)]);

    assert_ne!(
        refused.code,
        0,
        "a hook that judges an index reads no message, so there is nothing to defer to: {}",
        refused.output()
    );
    assert!(
        refused.output().contains("weeder guard refused"),
        "pre-commit refuses on its own:\n{}",
        refused.output()
    );
    assert_eq!(repo.head(), before, "no commit was made");
}

#[test]
fn pre_commit_defers_where_weeder_installed_nothing_at_all() {
    // The stem's own repository: it set core.hooksPath itself, wrote the one
    // hook its manifest declares, and weeder's install never ran, so there is no
    // record of weeder's for the recognition to lean on.
    let repo = Repo::init();
    repo.git(&["config", "core.hooksPath", ".plotplot/hooks"]);
    stem_binary(&repo);
    write_hook(&repo, ".plotplot/hooks/commit-msg", &the_stage());
    repo.write(".claude/settings.json", "{}\n");
    repo.git(&["add", ".claude/settings.json"]);

    let deferred = repo.weeder(&["guard", "pre-commit"]);

    assert_eq!(
        deferred.code, 0,
        "the stage is where git looks, so pre-commit hands it the verdict\n{}{}",
        deferred.stdout, deferred.stderr
    );
    assert!(
        deferred.stdout.contains(RULE),
        "pre-commit still prints the finding it is deferring:\n{}",
        deferred.stdout
    );

    std::fs::remove_file(repo.root().join(".plotplot/hooks/commit-msg"))
        .expect("the hook is the test's to take away");

    let refused = repo.weeder(&["guard", "pre-commit"]);

    assert_eq!(
        refused.code, 2,
        "with no stage anywhere, pre-commit is the gate again\n{}{}",
        refused.stdout, refused.stderr
    );
}

#[test]
fn status_names_the_stage_another_planter_wrote_and_is_content_with_it() {
    let repo = planted(&the_stage());

    let run = repo.weeder(&["guard", "status"]);

    assert_eq!(
        run.code, 0,
        "a stage git runs is a stage, whoever wrote it\n{}{}",
        run.stdout, run.stderr
    );
    assert!(
        run.stdout
            .lines()
            .any(|line| line.contains("commit-msg") && line.contains("planter")),
        "status says who the hook belongs to rather than calling it a miss:\n{}",
        run.stdout
    );
    for hook in ["pre-commit", "pre-push", "pre-rebase"] {
        assert!(
            run.stdout
                .lines()
                .any(|line| line.contains(hook) && line.contains("live")),
            "status still reports {hook} live:\n{}",
            run.stdout
        );
    }
}

#[test]
fn uninstall_takes_only_the_files_weeders_marker_names() {
    let repo = planted(&the_stage());

    let run = repo.weeder(&["guard", "uninstall"]);
    assert_eq!(run.code, 0, "uninstall runs clean\n{}", run.stderr);

    assert_eq!(
        std::fs::read_to_string(repo.root().join(STAGE)).expect("the stage is on disk"),
        the_stage(),
        "a hook another planter wrote is not weeder's to take away"
    );
    for hook in ["pre-commit", "pre-push", "pre-rebase"] {
        assert!(
            !repo.root().join(".githooks").join(hook).exists(),
            "the hooks weeder did write are gone"
        );
    }
    assert!(
        run.stdout.contains("commit-msg"),
        "uninstall says which file it left behind:\n{}",
        run.stdout
    );
}

/// A repository with weeder's four hooks installed and committed, and the
/// commit-msg one replaced by the hook another planter wrote there.
fn planted(hook: &str) -> Repo {
    let repo = Repo::init();
    let run = repo.weeder(&["guard", "install"]);
    assert_eq!(run.code, 0, "install runs clean\n{}", run.stderr);
    repo.commit("weeder guard installed");
    stem_binary(&repo);
    write_hook(&repo, STAGE, hook);
    repo
}

/// The binary the stem's hook names, at the path the stem keeps it. It is the
/// weeder under test, reached by a path weeder's own install would never write.
fn stem_binary(repo: &Repo) {
    let path = repo.root().join(STEM_BINARY);
    let directory = path.parent().expect("the binary sits in a directory");
    std::fs::create_dir_all(directory).expect("a directory for the binary");
    common::install_script(
        directory,
        "weeder",
        &format!(
            "#!/bin/sh\nexec {} \"$@\"\n",
            common::shell_word(&common::binary().display().to_string())
        ),
    );
}

fn write_hook(repo: &Repo, path: &str, body: &str) {
    repo.write(path, body);
    let path = repo.root().join(path);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("the hook should be runnable");
    assert!(Path::new(&path).is_file(), "the hook is on disk");
}

/// The guardrail change staged, and only it: the replaced hook stays in the
/// working tree, where a commit does not reach it.
fn staged(repo: &Repo) {
    repo.write(GUARDRAIL, PLANTED);
    repo.git(&["add", GUARDRAIL]);
}

/// A message with its trailer, as a person writes one.
fn allowing(rule: &str, reason: &str) -> String {
    format!("plant the repository's own hook\n\nWeeder-allow: {rule} {reason}\n")
}
