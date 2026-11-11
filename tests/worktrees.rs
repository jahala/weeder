//! A helper's worktree goes when its branch merges (#133).
//!
//! The lead hands work to helper agents, each in a worktree under
//! `.claude/worktrees/`, and each worktree builds a `target/` of its own, three
//! to four gigabytes. On 2026-11-07 ten of them sat merged and clean beside the
//! main checkout, and the disk was down to 13 GB. `scripts/check/worktrees.sh`
//! fails while one is left behind, and `scripts/worktrees-prune.sh` removes
//! exactly those: clean, and merged into the branch the work lands on. A
//! worktree with work in it, or one whose branch has not landed, stays.

mod common;

use std::path::{Path, PathBuf};

use common::Repo;

/// One of the two scripts, run on a repository, with the branch work lands on
/// named, since a test repository has no `origin/master`.
fn script(name: &str, repo: &Repo) -> (i32, String) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join(name);
    // What the prune moves aside stays inside the repository's own temporary
    // directory, which the test removes, rather than in the owner's trash.
    let aside = repo.root().join(".removed");
    std::fs::create_dir_all(&aside).expect("a folder to move worktrees into");
    let output = common::bash(&path)
        .arg(repo.root())
        .env("WEEDER_MERGED_INTO", "main")
        .env("WEEDER_TRASH_INTO", &aside)
        .output()
        .expect("bash should run the script");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.code().unwrap_or(-1), said)
}

/// A helper's worktree on a branch of its own, with one commit in it.
fn helper(repo: &Repo, name: &str) -> PathBuf {
    let tree = repo.root().join(".claude/worktrees").join(name);
    let path = tree.to_string_lossy().into_owned();
    repo.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        &format!("helper/{name}"),
        &path,
    ]);
    std::fs::write(tree.join(format!("{name}.txt")), "the helper's work\n")
        .expect("the helper writes its work");
    repo.git(&["-C", &path, "add", &format!("{name}.txt")]);
    repo.git(&["-C", &path, "commit", "-q", "-m", "the helper's work"]);
    tree
}

#[test]
fn a_helper_worktree_left_behind_after_its_branch_merges_fails_the_check_until_the_prune_removes_it(
) {
    let repo = Repo::init();
    let merged = helper(&repo, "agent-merged");
    let open = helper(&repo, "agent-open");
    repo.git(&[
        "merge",
        "-q",
        "--no-ff",
        "helper/agent-merged",
        "-m",
        "the helper's work lands",
    ]);

    let (code, said) = script("check/worktrees.sh", &repo);
    assert_eq!(
        code, 1,
        "a merged, clean helper worktree is left behind: {said}"
    );
    assert!(said.contains("agent-merged"), "the check names it: {said}");
    assert!(
        !said.contains("agent-open"),
        "a branch that has not landed is not left behind: {said}"
    );

    let (code, said) = script("worktrees-prune.sh", &repo);
    assert_eq!(code, 0, "the prune runs: {said}");
    assert!(
        !merged.exists(),
        "the merged helper's worktree is gone: {said}"
    );
    assert!(open.exists(), "the open helper's worktree stays: {said}");
    let listed = repo.git(&["worktree", "list"]);
    assert!(
        !listed.contains("agent-merged"),
        "git no longer lists it: {listed}"
    );

    let (code, said) = script("check/worktrees.sh", &repo);
    assert_eq!(code, 0, "nothing merged is left behind: {said}");
}

#[test]
fn a_merged_helper_worktree_with_work_in_it_is_reported_and_never_removed() {
    let repo = Repo::init();
    let merged = helper(&repo, "agent-dirty");
    repo.git(&[
        "merge",
        "-q",
        "--no-ff",
        "helper/agent-dirty",
        "-m",
        "the helper's work lands",
    ]);
    std::fs::write(merged.join("unsaved.txt"), "work nobody committed\n").expect("a file is left");

    let (code, said) = script("worktrees-prune.sh", &repo);
    assert_eq!(code, 1, "the prune says it left one: {said}");
    assert!(
        merged.join("unsaved.txt").exists(),
        "uncommitted work is never removed: {said}"
    );
    assert!(
        said.contains("agent-dirty"),
        "the prune names what it left: {said}"
    );
}
