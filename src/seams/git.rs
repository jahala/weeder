//! The git seam: everything weed knows about a repository, one function per
//! question. Every call is an argument array handed to `git`, never a shell
//! string, so a path or a ref that looks like a flag or a pipe is data.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::seams::fs;

/// The tree object every git repository shares for "nothing at all". A
/// repository before its first commit has no `HEAD`, and this is how git itself
/// spells the state such a repository is diffed against.
pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// The number of context lines weed asks for. Three is git's own default, and
/// rules that read the lines around a change get the same view a reviewer does.
const CONTEXT_LINES: &str = "--unified=3";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    NotARepository { path: String },
    UnknownRef { reference: String },
    Refused { command: String, message: String },
    Unavailable { message: String },
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::NotARepository { path } => write!(
                f,
                "{path} is not inside a git repository, so there is no diff to judge. run weed from a working tree."
            ),
            GitError::UnknownRef { reference } => write!(
                f,
                "this repository resolves no ref named {reference}. pass --base a ref it has, such as origin/main."
            ),
            GitError::Refused { command, message } => {
                write!(f, "git {command} failed: {message}.")
            }
            GitError::Unavailable { message } => {
                write!(f, "weed could not run git: {message}. install git, or put it on PATH.")
            }
        }
    }
}

impl std::error::Error for GitError {}

/// The working tree a path sits in. git refuses `rev-parse --show-toplevel`
/// when the path is outside every repository, and that is the only refusal this
/// question has.
pub fn repository_root(start: &Path) -> Result<PathBuf, GitError> {
    let output = run(start, &["rev-parse", "--show-toplevel"]).map_err(|error| match error {
        GitError::Refused { .. } => GitError::NotARepository {
            path: start.display().to_string(),
        },
        other => other,
    })?;
    Ok(PathBuf::from(output.trim_end()))
}

/// The commit a ref names, or `UnknownRef` when the repository has no such ref.
pub fn resolve_ref(root: &Path, reference: &str) -> Result<String, GitError> {
    let revision = format!("{reference}^{{commit}}");
    match run(root, &["rev-parse", "--verify", "--quiet", &revision]) {
        Ok(output) => Ok(output.trim_end().to_string()),
        Err(GitError::Refused { .. }) => Err(GitError::UnknownRef {
            reference: reference.to_string(),
        }),
        Err(other) => Err(other),
    }
}

/// Whether a ref resolves at all.
pub fn has_ref(root: &Path, reference: &str) -> Result<bool, GitError> {
    match resolve_ref(root, reference) {
        Ok(_) => Ok(true),
        Err(GitError::UnknownRef { .. }) => Ok(false),
        Err(other) => Err(other),
    }
}

/// The index and the working tree against `HEAD` — everything a worker changed,
/// staged or not. This is what pleach's smoke gate sees.
pub fn diff_head(root: &Path) -> Result<String, GitError> {
    let head = head_or_empty_tree(root)?;
    diff(root, &[&head])
}

/// The index alone against `HEAD` — what a commit would carry.
pub fn diff_index(root: &Path) -> Result<String, GitError> {
    let head = head_or_empty_tree(root)?;
    diff(root, &["--cached", &head])
}

/// The tree against a ref — what a branch changed, as CI reads it.
pub fn diff_ref(root: &Path, reference: &str) -> Result<String, GitError> {
    let commit = resolve_ref(root, reference)?;
    diff(root, &[&commit])
}

/// A file's contents at a ref, or `None` where the ref does not carry it.
pub fn file_at_ref(root: &Path, reference: &str, path: &str) -> Result<Option<String>, GitError> {
    let object = format!("{reference}:{path}");
    match run(root, &["show", &object]) {
        Ok(contents) => Ok(Some(contents)),
        Err(GitError::Refused { .. }) => Ok(None),
        Err(other) => Err(other),
    }
}

/// A file's contents in the working tree, or `None` where there is no such file.
pub fn file_in_tree(root: &Path, path: &str) -> Result<Option<String>, GitError> {
    fs::read_if_present(&root.join(path)).map_err(|error| GitError::Refused {
        command: format!("show :{path}"),
        message: error.message,
    })
}

/// The messages of the commits a range carries, newest first, for the trailers
/// an author wrote there.
pub fn commit_messages(root: &Path, base: &str) -> Result<Vec<String>, GitError> {
    let commit = resolve_ref(root, base)?;
    if !has_ref(root, "HEAD")? {
        return Ok(Vec::new());
    }
    let range = format!("{commit}..HEAD");
    let output = run(root, &["log", "--format=%B%x00", &range])?;
    Ok(output
        .split('\u{0}')
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToString::to_string)
        .collect())
}

/// The message of the commit being prepared, where git keeps it. A pre-commit
/// gate reads its trailers from here because the commit does not exist yet.
pub fn pending_commit_message(root: &Path) -> Result<Option<String>, GitError> {
    let path = run(root, &["rev-parse", "--git-path", "COMMIT_EDITMSG"])?;
    let path = root.join(path.trim_end());
    fs::read_if_present(&path).map_err(|error| GitError::Refused {
        command: "rev-parse --git-path COMMIT_EDITMSG".to_string(),
        message: error.message,
    })
}

/// Where this repository looks for its hooks.
pub fn hooks_path(root: &Path) -> Result<PathBuf, GitError> {
    let path = run(root, &["rev-parse", "--git-path", "hooks"])?;
    Ok(root.join(path.trim_end()))
}

fn head_or_empty_tree(root: &Path) -> Result<String, GitError> {
    match resolve_ref(root, "HEAD") {
        Ok(commit) => Ok(commit),
        Err(GitError::UnknownRef { .. }) => Ok(EMPTY_TREE.to_string()),
        Err(other) => Err(other),
    }
}

fn diff(root: &Path, revisions: &[&str]) -> Result<String, GitError> {
    let mut arguments = vec![
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "--find-renames",
        CONTEXT_LINES,
    ];
    arguments.extend_from_slice(revisions);
    run(root, &arguments)
}

/// One git invocation. Its config comes from the repository alone: a global
/// `quotepath` or an external diff driver must not change what weed judges.
fn run(directory: &Path, arguments: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory.as_os_str())
        .args(["-c", "core.quotepath=false"])
        .args(arguments.iter().map(OsStr::new))
        .output()
        .map_err(|error| GitError::Unavailable {
            message: error.to_string(),
        })?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|error| GitError::Refused {
            command: arguments.join(" "),
            message: error.to_string(),
        })
    } else {
        Err(GitError::Refused {
            command: arguments.join(" "),
            message: first_line(&String::from_utf8_lossy(&output.stderr)),
        })
    }
}

fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("git said nothing")
        .to_string()
}
