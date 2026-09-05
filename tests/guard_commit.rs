//! The installed pre-commit hook, refusing an index that blocks.
//!
//! Nothing here calls weed directly once the hooks are in: the test runs
//! `git commit` and reads what git did, which is the only proof that the law is
//! in git rather than in a wrapper somebody can decline to call.

mod common;

use common::{conflicted_parser, Repo};

/// The parser once the merge is finished: one side kept, no marker left.
const RESOLVED: &str = "export function parse(input: string): string[] {\n  \
                        return input.split(\";\");\n}\n";

#[test]
fn the_pre_commit_hook_refuses_an_index_that_blocks_and_lets_a_clean_one_through() {
    let repo = Repo::init();
    repo.weed(&["guard", "install"]);
    let before = repo.head();

    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();
    let refused = repo.try_git(&["commit", "-m", "the merge, half finished"]);

    assert_ne!(
        refused.code,
        0,
        "git stops when its pre-commit hook refuses: {}",
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
        "the hook says which gate refused and what to do:\n{said}"
    );
    assert_eq!(
        repo.head(),
        before,
        "no commit was made, so the marker never reached the history"
    );

    repo.write("src/parser.ts", RESOLVED);
    repo.stage_all();
    let allowed = repo.try_git(&["commit", "-m", "the merge, finished"]);

    assert_eq!(
        allowed.code,
        0,
        "a clean index commits: {}",
        allowed.output()
    );
    assert_ne!(repo.head(), before, "the commit is really there");
}

#[test]
fn the_pre_commit_hook_judges_the_index_and_not_the_working_tree() {
    let repo = Repo::init();
    repo.weed(&["guard", "install"]);

    repo.write("src/parser.ts", RESOLVED);
    repo.stage_all();
    // Half a merge, never staged. A commit carries the index, so this is not
    // what the hook is judging, and refusing it would refuse honest work.
    repo.write("src/reader.ts", &conflicted_parser(None));

    let allowed = repo.try_git(&["commit", "-m", "the parser"]);

    assert_eq!(
        allowed.code,
        0,
        "what is not in the index is not what the commit carries: {}",
        allowed.output()
    );
}
