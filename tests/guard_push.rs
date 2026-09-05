//! The installed pre-push hook, against a real bare remote.
//!
//! The remote is a bare repository on disk, so every assertion about what was
//! refused is an assertion about what the remote does or does not now hold.

mod common;

use common::{conflicted_parser, Repo};

const RESOLVED: &str = "export function parse(input: string): string[] {\n  \
                        return input.split(\";\");\n}\n";
const WIDENED: &str = "export function parse(input: string): string[] {\n  \
                       return input.split(/[,;]/);\n}\n";
const NARROWED: &str = "export function parse(input: string): string[] {\n  \
                        return input.split(\",\");\n}\n";

#[test]
fn the_pre_push_hook_refuses_a_non_fast_forward_to_a_protected_branch() {
    let (repo, remote) = guarded();
    let held = remote.git(&["rev-parse", "main"]).trim().to_string();

    repo.write("src/parser.ts", WIDENED);
    repo.stage_all();
    repo.git(&["commit", "--amend", "-m", "the parser, rewritten"]);
    let refused = repo.try_git(&["push", "--force", "origin", "main"]);

    assert_ne!(
        refused.code,
        0,
        "git stops when its pre-push hook refuses: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(
        said.contains("weed guard refused"),
        "the hook says which gate refused:\n{said}"
    );
    assert!(
        said.contains("fast-forward"),
        "the hook says what is wrong with the push:\n{said}"
    );
    assert!(said.contains("main"), "the hook names the branch:\n{said}");
    assert_eq!(
        remote.git(&["rev-parse", "main"]).trim(),
        held,
        "the remote still holds the commits the force would have dropped"
    );
}

#[test]
fn the_pre_push_hook_refuses_a_range_that_carries_a_finding_that_blocks() {
    let (repo, remote) = published();
    let held = remote.git(&["rev-parse", "main"]).trim().to_string();

    // Committed before the hooks are in, which is how such a commit gets made
    // at all, and exactly the history a push is the last chance to stop.
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.commit("the merge, half finished");
    repo.weed(&["guard", "install"]);

    let refused = repo.try_git(&["push", "origin", "main"]);

    assert_ne!(
        refused.code,
        0,
        "git stops when its pre-push hook refuses: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(said.contains("G1"), "the hook prints the findings:\n{said}");
    assert!(
        said.contains("src/parser.ts"),
        "the hook names the file:\n{said}"
    );
    assert!(
        said.contains("weed guard refused"),
        "the hook says which gate refused:\n{said}"
    );
    assert_eq!(
        remote.git(&["rev-parse", "main"]).trim(),
        held,
        "the marker never reached the remote"
    );
}

#[test]
fn the_pre_push_hook_lets_a_clean_fast_forward_through() {
    let (repo, remote) = guarded();

    repo.write("src/parser.ts", WIDENED);
    repo.commit("the parser, widened");
    let allowed = repo.try_git(&["push", "origin", "main"]);

    assert_eq!(
        allowed.code,
        0,
        "honest work goes through: {}",
        allowed.output()
    );
    assert_eq!(
        remote.git(&["rev-parse", "main"]).trim(),
        repo.head(),
        "the remote carries what was pushed"
    );
}

#[test]
fn a_branch_that_is_not_protected_is_yours_to_rewrite() {
    let (repo, _remote) = guarded();

    repo.git(&["checkout", "-b", "feature/split"]);
    repo.write("src/parser.ts", WIDENED);
    repo.commit("the split");
    let first = repo.try_git(&["push", "origin", "feature/split"]);
    assert_eq!(
        first.code,
        0,
        "a branch the remote has never seen is judged from where its own commits begin: {}",
        first.output()
    );

    repo.write("src/parser.ts", NARROWED);
    repo.stage_all();
    repo.git(&["commit", "--amend", "-m", "the split, rewritten"]);
    let rewritten = repo.try_git(&["push", "--force", "origin", "feature/split"]);

    assert_eq!(
        rewritten.code,
        0,
        "only a protected branch is kept from being rewritten: {}",
        rewritten.output()
    );
}

/// The same repository with weed's own hooks installed and published along with
/// it, which is the state a project is in once it has adopted weed.
///
/// The hooks live in the working tree so a clone gets them, and a hook is a
/// guardrail, so the first commit and the first push that carry them are C1
/// findings the gate itself refuses. Adopting weed is therefore one deliberate
/// commit and one deliberate push that go around it, and both happen here,
/// before there is any published history to protect.
fn guarded() -> (Repo, Repo) {
    let remote = Repo::bare();
    let repo = Repo::init();
    repo.weed(&["guard", "install"]);
    repo.stage_all();
    repo.git(&["commit", "--no-verify", "-m", "weed guard installed"]);
    publish(repo, remote, Adoption::Skipped)
}

/// A repository whose main branch is already on a bare remote, with no hooks in
/// it yet. Both temp directories must outlive the test, so both come back.
fn published() -> (Repo, Repo) {
    publish(Repo::init(), Repo::bare(), Adoption::None)
}

/// Whether the history being published already carries weed's own hooks.
enum Adoption {
    None,
    Skipped,
}

/// One honest commit, and the branch that carries it pushed to the remote.
fn publish(repo: Repo, remote: Repo, adoption: Adoption) -> (Repo, Repo) {
    repo.write("src/parser.ts", RESOLVED);
    repo.commit("the parser");
    repo.git(&[
        "remote",
        "add",
        "origin",
        &remote.root().display().to_string(),
    ]);
    let mut push = vec!["push", "--set-upstream", "origin", "main"];
    if matches!(adoption, Adoption::Skipped) {
        push.insert(1, "--no-verify");
    }
    repo.git(&push);
    (repo, remote)
}
