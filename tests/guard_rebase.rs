//! The installed pre-rebase hook, keeping protected history from being rewritten.

mod common;

use common::Repo;

const RESOLVED: &str = "export function parse(input: string): string[] {\n  \
                        return input.split(\";\");\n}\n";

#[test]
fn the_pre_rebase_hook_refuses_rewriting_the_branch_that_is_checked_out() {
    let repo = branched();
    repo.git(&["checkout", "main"]);
    let before = repo.head();

    let refused = repo.try_git(&["rebase", "feature/split"]);

    assert_ne!(
        refused.code,
        0,
        "git stops when its pre-rebase hook refuses: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(
        said.contains("weed guard refused"),
        "the hook says which gate refused:\n{said}"
    );
    assert!(said.contains("main"), "the hook names the branch:\n{said}");
    assert_eq!(
        repo.head(),
        before,
        "main still holds exactly the commits it held"
    );
}

#[test]
fn the_pre_rebase_hook_refuses_a_protected_branch_named_on_the_command_line() {
    let repo = branched();
    repo.git(&["checkout", "feature/split"]);
    let before = repo.git(&["rev-parse", "main"]).trim().to_string();

    // git rebase <upstream> <branch> checks the branch out first and rewrites
    // it, so the branch being rewritten is the argument and not what is checked
    // out when the rebase begins.
    let refused = repo.try_git(&["rebase", "feature/split", "main"]);

    assert_ne!(
        refused.code,
        0,
        "the branch a rebase is given is the one it rewrites: {}",
        refused.output()
    );
    assert!(
        refused.output().contains("weed guard refused"),
        "the hook says which gate refused:\n{}",
        refused.output()
    );
    assert_eq!(
        repo.git(&["rev-parse", "main"]).trim(),
        before,
        "main still holds exactly the commits it held"
    );
}

#[test]
fn the_pre_rebase_hook_lets_a_branch_of_your_own_be_rebased() {
    let repo = branched();
    repo.git(&["checkout", "feature/split"]);
    let onto = repo.git(&["rev-parse", "main"]).trim().to_string();

    let allowed = repo.try_git(&["rebase", "main"]);

    assert_eq!(
        allowed.code,
        0,
        "rebasing your own branch onto a protected one is the ordinary case: {}",
        allowed.output()
    );
    assert_eq!(
        repo.git(&["rev-parse", "HEAD~1"]).trim(),
        onto,
        "the branch really was rebased onto main"
    );
}

/// A repository with a protected branch and a branch of one's own, each carrying
/// a commit the other does not, so a rebase between them is a real rebase. The
/// hooks go in last, since the setup itself is honest work.
fn branched() -> Repo {
    let repo = Repo::init();
    repo.write("src/parser.ts", RESOLVED);
    repo.commit("the parser");

    repo.git(&["checkout", "-b", "feature/split"]);
    repo.write("src/split.ts", "export const split = \";\";\n");
    repo.commit("the split");

    repo.git(&["checkout", "main"]);
    repo.write("src/reader.ts", "export const reader = 1;\n");
    repo.commit("the reader");

    repo.weed(&["guard", "install"]);
    repo
}
