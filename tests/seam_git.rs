//! The git seam: the questions weed asks a repository, and the answers it gets.
//!
//! Every case runs against a real repository in a temp directory. The seam is
//! not a face, so these tests call it directly; nothing about git is simulated.

mod common;

use common::Repo;
use weed::seams::git::{self, GitError};

#[test]
fn the_root_is_the_working_tree_a_path_sits_in() {
    let repo = Repo::init();
    repo.write("src/deep/file.ts", "export const x = 1;\n");
    repo.commit("a file, some way down");

    let from_below = git::repository_root(&repo.root().join("src/deep"))
        .expect("a path inside a repository has a root");
    let from_the_top = git::repository_root(repo.root()).expect("so does the root itself");
    assert_eq!(from_below, from_the_top);
    assert!(from_the_top.join(".git").exists());
}

#[test]
fn a_path_outside_every_repository_is_not_a_repository() {
    let elsewhere = tempfile::TempDir::new().expect("a directory that is not a repository");
    let error = git::repository_root(elsewhere.path()).expect_err("there is no repository here");

    assert!(matches!(error, GitError::NotARepository { .. }));
    assert!(
        error.to_string().contains("git repository"),
        "the error says what was missing: {error}"
    );
}

#[test]
fn a_ref_resolves_or_it_does_not() {
    let repo = Repo::init();
    let head = repo.head();

    assert_eq!(
        git::resolve_ref(repo.root(), "HEAD").expect("HEAD resolves"),
        head
    );
    assert!(git::has_ref(repo.root(), "HEAD").expect("asking is not an error"));
    assert!(!git::has_ref(repo.root(), "origin/nowhere").expect("asking is not an error"));

    let error = git::resolve_ref(repo.root(), "origin/nowhere").expect_err("there is no such ref");
    assert!(matches!(error, GitError::UnknownRef { .. }));
    assert!(error.to_string().contains("origin/nowhere"));
}

#[test]
fn each_mode_asks_git_for_its_own_diff() {
    let repo = Repo::init();
    repo.write("tracked.ts", "export const kept = true;\n");
    repo.commit("the base");
    let base = repo.head();

    repo.write("committed.ts", "export const landed = true;\n");
    repo.commit("a commit past the base");
    repo.write("staged.ts", "export const staged = true;\n");
    repo.git(&["add", "staged.ts"]);
    repo.write("tracked.ts", "export const kept = false;\n");

    let head = git::diff_head(repo.root()).expect("the index and the tree against HEAD");
    assert!(head.contains("staged.ts") && head.contains("tracked.ts"));
    assert!(!head.contains("committed.ts"));

    let index = git::diff_index(repo.root()).expect("the index alone");
    assert!(index.contains("staged.ts"));
    assert!(!index.contains("tracked.ts"));

    let against_base = git::diff_ref(repo.root(), &base).expect("the tree against a ref");
    for path in ["committed.ts", "staged.ts", "tracked.ts"] {
        assert!(against_base.contains(path), "{path} is past the base");
    }
}

#[test]
fn a_rename_is_a_rename_and_not_a_deletion_with_an_addition() {
    let repo = Repo::init();
    repo.write(
        "src/parser.ts",
        "export const parse = (input: string) => input;\n",
    );
    repo.commit("the file, under its first name");
    repo.git(&["mv", "src/parser.ts", "src/reader.ts"]);

    let diff = git::diff_head(repo.root()).expect("the diff reads");
    assert!(diff.contains("rename from src/parser.ts"));
    assert!(diff.contains("rename to src/reader.ts"));
}

#[test]
fn a_repository_before_its_first_commit_is_diffed_against_the_empty_tree() {
    let repo = Repo::init();
    repo.git(&["update-ref", "-d", "HEAD"]);
    repo.write("new.ts", "export const first = true;\n");
    repo.stage_all();

    let diff = git::diff_head(repo.root()).expect("there is still a diff to read");
    assert!(diff.contains("new.ts"));
    assert!(diff.contains("new file mode"), "everything here is new");
}

#[test]
fn a_file_is_read_at_a_ref_and_in_the_tree() {
    let repo = Repo::init();
    repo.write("src/parser.ts", "export const version = 1;\n");
    repo.commit("version one");
    repo.write("src/parser.ts", "export const version = 2;\n");

    assert_eq!(
        git::file_at_ref(repo.root(), "HEAD", "src/parser.ts").expect("HEAD carries it"),
        Some("export const version = 1;\n".to_string())
    );
    assert_eq!(
        git::file_in_tree(repo.root(), "src/parser.ts").expect("the tree carries it"),
        Some("export const version = 2;\n".to_string())
    );
    assert_eq!(
        git::file_at_ref(repo.root(), "HEAD", "src/absent.ts").expect("asking is not an error"),
        None
    );
    assert_eq!(
        git::file_in_tree(repo.root(), "src/absent.ts").expect("asking is not an error"),
        None
    );
}

#[test]
fn the_messages_of_a_range() {
    let repo = Repo::init();
    repo.write("a.ts", "export const a = 1;\n");
    repo.commit("the base");
    let base = repo.head();

    repo.write("b.ts", "export const b = 2;\n");
    repo.commit("the second change\n\nWeed-allow: G1 the merge finishes next commit");
    repo.write("c.ts", "export const c = 3;\n");
    repo.commit("the third change");

    let messages =
        git::commit_messages(repo.root(), &base, "HEAD").expect("the range has messages");
    assert_eq!(
        messages.len(),
        2,
        "the base's own message is not in the range"
    );
    assert!(messages
        .iter()
        .any(|message| message.contains("Weed-allow: G1")));
}

#[test]
fn the_hooks_path_is_where_this_repository_looks_for_its_hooks() {
    let repo = Repo::init();
    let hooks = git::hooks_path(repo.root()).expect("every repository has one");

    assert!(hooks.ends_with("hooks"));
    assert!(hooks.starts_with(repo.root()));
}
