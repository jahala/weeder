//! What `weed check` judges, and against what.
//!
//! The repository below carries a distinct conflict in each zone: one in a
//! commit past the base ref, one in the index, one in the working tree alone.
//! Each mode reports exactly the zones it claims to read, so the paths in the
//! SARIF say which diff weed asked git for.

mod common;

use common::{conflicted_parser, Repo};

/// A repository with a conflict in every zone, and the ref before all of them.
fn three_zones() -> (Repo, String) {
    let conflict = conflicted_parser(None);
    let repo = Repo::init();
    repo.write("tracked.ts", "export const kept = true;\n");
    repo.commit("the state CI compares against");
    let base = repo.head();

    repo.write("committed.ts", &conflict);
    repo.commit("a conflict that reached a commit");

    repo.write("staged.ts", &conflict);
    repo.write("src/nested.ts", &conflict);
    repo.git(&["add", "staged.ts", "src/nested.ts"]);

    repo.write("tracked.ts", &conflict);

    (repo, base)
}

#[test]
fn with_no_flags_weed_judges_the_index_and_the_working_tree_against_head() {
    let (repo, _) = three_zones();
    let run = repo.weed(&["check"]);

    assert_eq!(
        run.paths(),
        vec!["src/nested.ts", "staged.ts", "tracked.ts"],
        "staged and unstaged changes are judged; what HEAD already carries is not"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn staged_judges_the_index_alone() {
    let (repo, _) = three_zones();
    let run = repo.weed(&["check", "--staged"]);

    assert_eq!(
        run.paths(),
        vec!["src/nested.ts", "staged.ts"],
        "a pre-commit gate sees what the commit would carry, and nothing else"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn base_judges_the_tree_against_that_ref() {
    let (repo, base) = three_zones();
    let run = repo.weed(&["check", "--base", &base]);

    assert_eq!(
        run.paths(),
        vec!["committed.ts", "src/nested.ts", "staged.ts", "tracked.ts"],
        "CI sees every zone: the commits since the base, the index, and the tree"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn scope_restricts_the_judged_paths() {
    let (repo, base) = three_zones();

    let run = repo.weed(&["check", "--scope", "src/**"]);
    assert_eq!(run.paths(), vec!["src/nested.ts"]);

    let run = repo.weed(&["check", "--base", &base, "--scope", "*.ts"]);
    assert_eq!(
        run.paths(),
        vec!["committed.ts", "staged.ts", "tracked.ts"],
        "a single star stops at a separator, so it never reaches into src/"
    );

    let run = repo.weed(&["check", "--scope", "**/*.ts", "--scope", "docs/**"]);
    assert_eq!(
        run.paths(),
        vec!["src/nested.ts", "staged.ts", "tracked.ts"],
        "several scopes allow a path any one of them allows"
    );
}

#[test]
fn base_and_staged_together_are_two_questions_so_weed_asks_for_one() {
    let (repo, base) = three_zones();
    let run = repo.weed(&["check", "--base", &base, "--staged"]);

    assert_eq!(
        run.code, 3,
        "a gate that cannot tell what to judge fails closed"
    );
    assert_eq!(run.stderr_lines().len(), 1, "one line, naming the cause");
}

#[test]
fn a_repository_before_its_first_commit_still_has_its_change_judged() {
    let repo = Repo::init();
    repo.git(&["update-ref", "-d", "HEAD"]);
    repo.write("staged.ts", &conflicted_parser(None));
    repo.stage_all();

    let run = repo.weed(&["check"]);
    assert_eq!(
        run.paths(),
        vec!["staged.ts"],
        "with no HEAD to compare against, everything in the tree is new"
    );
    assert_eq!(run.code, 2);
}
