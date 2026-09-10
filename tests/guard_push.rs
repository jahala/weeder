//! The installed pre-push hook, against a real bare remote.
//!
//! The remote is a bare repository on disk, so every assertion about what was
//! refused is an assertion about what the remote does or does not now hold.
//!
//! A push carries commits, and a commit carries the message a person wrote on
//! it, so the allowance commit-msg honoured travels with the change all the way
//! here and is honoured again. A marker on a line travels too, and is not: the
//! line is the agent's to write, which is the whole of why the two differ.

mod common;

use common::{conflicted_parser, Repo};

const RESOLVED: &str = "export function parse(input: string): string[] {\n  \
                        return input.split(\";\");\n}\n";
const WIDENED: &str = "export function parse(input: string): string[] {\n  \
                       return input.split(/[,;]/);\n}\n";
const NARROWED: &str = "export function parse(input: string): string[] {\n  \
                        return input.split(\",\");\n}\n";

/// A guardrail path a repository plants a hook of its own at, beside the four
/// weeder wrote. It is a shell script, so the inline case has somewhere to write
/// a marker.
const GUARDRAIL: &str = ".githooks/post-commit";
const PLANTED: &str = "#!/bin/sh\n# the stem's own hook, planted beside weeder's\nexit 0\n";
const RULE: &str = "C1";
const REASON: &str = "the stem plants its own hook, and the owner asked for it";

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
        said.contains("weeder guard refused"),
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
    repo.weeder(&["guard", "install"]);

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
        said.contains("weeder guard refused"),
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

/// The same repository with weeder's own hooks installed and published along with
/// it, which is the state a project is in once it has adopted weeder.
fn guarded() -> (Repo, Repo) {
    let remote = Repo::bare();
    let repo = Repo::init();
    repo.weeder(&["guard", "install"]);
    // The hooks are guardrail paths, and C1 knows weeder's own bundle byte for
    // byte, so the commit and the push that first carry them go through the
    // very hooks they install.
    repo.commit("weeder guard installed");
    publish(repo, remote)
}

/// A repository whose main branch is already on a bare remote, with no hooks in
/// it yet. Both temp directories must outlive the test, so both come back.
fn published() -> (Repo, Repo) {
    publish(Repo::init(), Repo::bare())
}

/// One honest commit, and the branch that carries it pushed to the remote.
fn publish(repo: Repo, remote: Repo) -> (Repo, Repo) {
    repo.write("src/parser.ts", RESOLVED);
    repo.commit("the parser");
    repo.git(&[
        "remote",
        "add",
        "origin",
        &remote.root().display().to_string(),
    ]);
    repo.git(&["push", "--set-upstream", "origin", "main"]);
    (repo, remote)
}

#[test]
fn the_pre_push_hook_honours_the_trailer_that_allowed_the_commit_at_commit_msg() {
    let (repo, remote) = guarded();

    repo.write(GUARDRAIL, PLANTED);
    repo.stage_all();
    let landed = repo.try_git(&[
        "commit",
        "-m",
        &format!("plant the stem's own hook\n\nWeeder-allow: {RULE} {REASON}\n"),
    ]);
    assert_eq!(
        landed.code,
        0,
        "the allowance lands the commit at commit-msg: {}",
        landed.output()
    );

    let allowed = repo.try_git(&["push", "origin", "main"]);

    assert_eq!(
        allowed.code,
        0,
        "a commit a person allowed on its own message can leave the machine: {}",
        allowed.output()
    );
    assert_eq!(
        remote.git(&["rev-parse", "main"]).trim(),
        repo.head(),
        "the remote carries the commit the trailer allowed"
    );
}

#[test]
fn the_pre_push_hook_refuses_a_commit_whose_only_allowance_is_a_marker_on_the_line() {
    let (repo, remote) = published();
    let held = remote.git(&["rev-parse", "main"]).trim().to_string();

    // Committed before the hooks are in, which is how a commit carrying nothing
    // but an inline allowance gets made at all.
    repo.write(
        GUARDRAIL,
        &format!("#!/bin/sh\n# weeder-allow {RULE}: {REASON}\nexit 0\n"),
    );
    repo.commit("plant the stem's own hook");
    repo.weeder(&["guard", "install"]);

    let refused = repo.try_git(&["push", "origin", "main"]);

    assert_ne!(
        refused.code,
        0,
        "an allowance the agent wrote itself is reported, never honoured: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(said.contains(RULE), "the hook prints the finding:\n{said}");
    assert!(
        said.contains("weeder guard refused"),
        "the hook says which gate refused:\n{said}"
    );
    assert_eq!(
        remote.git(&["rev-parse", "main"]).trim(),
        held,
        "the guardrail change never reached the remote"
    );
}
