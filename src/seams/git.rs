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

/// The index and the working tree against `HEAD`, everything a worker changed,
/// staged or not. This is what pleach's smoke gate sees.
pub fn diff_head(root: &Path) -> Result<String, GitError> {
    let head = head_or_empty_tree(root)?;
    diff(root, &[&head])
}

/// The index alone against `HEAD`, what a commit would carry.
pub fn diff_index(root: &Path) -> Result<String, GitError> {
    let head = head_or_empty_tree(root)?;
    diff(root, &["--cached", &head])
}

/// The tree against a ref, what a branch changed, as CI reads it.
pub fn diff_ref(root: &Path, reference: &str) -> Result<String, GitError> {
    let base = resolve_base(root, reference)?;
    diff(root, &[&base])
}

/// One ref against another, the commits a range carries, with the working tree
/// having nothing to say about it. This is the view a pre-push hook needs: it
/// judges what is being pushed, not whatever the checkout happens to hold.
pub fn diff_range(root: &Path, base: &str, tip: &str) -> Result<String, GitError> {
    let base = resolve_base(root, base)?;
    let tip = resolve_ref(root, tip)?;
    diff(root, &[&base, &tip])
}

/// A file's text at a ref: `None` where the ref does not carry the file, and
/// `None` where what it carries is not text. weed judges lines, and bytes that
/// are not text carry none; G2 is the rule that reports a binary blob.
pub fn file_at_ref(root: &Path, reference: &str, path: &str) -> Result<Option<String>, GitError> {
    let object = format!("{reference}:{path}");
    text(root, &["show", &object])
}

/// A file's text in the index: what a commit would carry. git spells the index
/// as a ref with no name in front of the colon.
pub fn file_in_index(root: &Path, path: &str) -> Result<Option<String>, GitError> {
    let object = format!(":{path}");
    text(root, &["show", &object])
}

/// A file's text in the working tree, or `None` where there is no such file.
pub fn file_in_tree(root: &Path, path: &str) -> Result<Option<String>, GitError> {
    fs::read_text_if_present(&root.join(path)).map_err(|error| GitError::Refused {
        command: format!("show :{path}"),
        message: error.message,
    })
}

/// What a git invocation wrote, where it wrote text and worked. A refusal, no
/// such object, and bytes that are not text both answer `None`, because both
/// mean there is nothing for a rule to read.
fn text(root: &Path, arguments: &[&str]) -> Result<Option<String>, GitError> {
    let attempt = attempt(root, arguments)?;
    if attempt.code != 0 {
        return Ok(None);
    }
    // git's own test for a binary blob: a NUL byte. Anything else is text a rule
    // can read, and a stray byte that is not utf-8 is replaced rather than
    // letting one latin-1 character make a file look like it is not there.
    if attempt.stdout.contains(&0) {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&attempt.stdout).into_owned()))
}

/// The messages of the commits a range carries, newest first, for the trailers
/// an author wrote there. A base of the empty tree names no commit to start
/// after, so the range is the tip's whole history.
pub fn commit_messages(root: &Path, base: &str, tip: &str) -> Result<Vec<String>, GitError> {
    let commit = resolve_base(root, base)?;
    if !has_ref(root, tip)? {
        return Ok(Vec::new());
    }
    let tip = resolve_ref(root, tip)?;
    let range = if commit == EMPTY_TREE {
        tip
    } else {
        format!("{commit}..{tip}")
    };
    // A commit message is whatever its author typed, and a trailer written in a
    // message with a byte weed cannot read still has to be honoured.
    let output = run_lossy(root, &["log", "--format=%B%x00", &range])?;
    Ok(output
        .split('\u{0}')
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToString::to_string)
        .collect())
}

/// Every file the tree holds, as git sees it: what is tracked, and what is
/// untracked and not ignored. A scan judges the repository as it is, so a file
/// a worker wrote and never staged counts, and a file the ignore rules hide is
/// not part of the tree at all.
pub fn tree_files(root: &Path) -> Result<Vec<String>, GitError> {
    let listed = run_lossy(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?;
    // `-z` so a path with a newline or a quote in it arrives whole. git lists
    // a path once per index entry, and a file staged in more than one is still
    // one file.
    let mut paths: Vec<String> = listed
        .split('\u{0}')
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
        .collect();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

/// When each line of a file was last touched, as seconds since the epoch, in
/// line order. A path git will not blame, untracked, or gone, has no ages,
/// and a rule that needs one reports nothing rather than guessing.
pub fn blame_line_times(root: &Path, path: &str) -> Result<Vec<i64>, GitError> {
    let arguments = ["blame", "--line-porcelain", "--", path];
    let attempt = attempt(root, &arguments)?;
    if attempt.code != 0 {
        return Ok(Vec::new());
    }
    // The porcelain form writes one header block per line of the file, and
    // `author-time` inside it. The file's own text arrives on lines opening
    // with a tab, so a source line that itself starts with `author-time` can
    // never be read as a header.
    Ok(String::from_utf8_lossy(&attempt.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("author-time "))
        .filter_map(|seconds| seconds.trim().parse::<i64>().ok())
        .collect())
}

/// Where this repository looks for its hooks.
pub fn hooks_path(root: &Path) -> Result<PathBuf, GitError> {
    let path = run(root, &["rev-parse", "--git-path", "hooks"])?;
    Ok(root.join(path.trim_end()))
}

/// Whether one commit is already in another's history. git answers this with an
/// exit code rather than a refusal, so a "no" is an answer and not a failure.
pub fn is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> Result<bool, GitError> {
    let arguments = ["merge-base", "--is-ancestor", ancestor, descendant];
    let attempt = attempt(root, &arguments)?;
    match attempt.code {
        0 => Ok(true),
        1 => Ok(false),
        _ => Err(attempt.refusal(&arguments)),
    }
}

/// The last commit two refs share, or `None` where they share none at all.
pub fn merge_base(root: &Path, left: &str, right: &str) -> Result<Option<String>, GitError> {
    let arguments = ["merge-base", left, right];
    let attempt = attempt(root, &arguments)?;
    match attempt.code {
        0 => Ok(Some(attempt.text(&arguments)?)),
        1 => Ok(None),
        _ => Err(attempt.refusal(&arguments)),
    }
}

/// The commit just outside what `tip` adds to everything the remotes already
/// carry, where a branch no remote has seen begins. `None` when the whole
/// history is still unpublished.
pub fn unpublished_boundary(root: &Path, tip: &str) -> Result<Option<String>, GitError> {
    let output = run(root, &["rev-list", "--boundary", tip, "--not", "--remotes"])?;
    Ok(output
        .lines()
        .find_map(|line| line.strip_prefix('-'))
        .map(|commit| commit.trim().to_string()))
}

/// The branch that is checked out, or `None` on a detached HEAD, which is no
/// branch at all.
pub fn current_branch(root: &Path) -> Result<Option<String>, GitError> {
    let arguments = ["symbolic-ref", "--quiet", "--short", "HEAD"];
    let attempt = attempt(root, &arguments)?;
    match attempt.code {
        0 => Ok(Some(attempt.text(&arguments)?)),
        1 => Ok(None),
        _ => Err(attempt.refusal(&arguments)),
    }
}

/// This repository's own directory, where weed keeps what must never reach a
/// commit and never travel with a clone.
pub fn git_dir(root: &Path) -> Result<PathBuf, GitError> {
    let path = run(root, &["rev-parse", "--absolute-git-dir"])?;
    Ok(PathBuf::from(path.trim_end()))
}

/// A setting in this repository's own configuration, or `None` where it carries
/// none. The repository's file alone is read: a setting a person keeps in their
/// global configuration is theirs, and weed must not put it back as if it were
/// this repository's.
pub fn config_get(root: &Path, key: &str) -> Result<Option<String>, GitError> {
    let arguments = ["config", "--local", "--get", key];
    let attempt = attempt(root, &arguments)?;
    match attempt.code {
        0 => Ok(Some(attempt.text(&arguments)?)),
        1 => Ok(None),
        _ => Err(attempt.refusal(&arguments)),
    }
}

/// A setting written into this repository's own configuration.
pub fn config_set(root: &Path, key: &str, value: &str) -> Result<(), GitError> {
    run(root, &["config", "--local", key, value]).map(|_| ())
}

/// A setting taken out of this repository's own configuration. A setting that
/// was not there is already gone; git spells that refusal 5.
pub fn config_unset(root: &Path, key: &str) -> Result<(), GitError> {
    let arguments = ["config", "--local", "--unset", key];
    let attempt = attempt(root, &arguments)?;
    match attempt.code {
        0 | 5 => Ok(()),
        _ => Err(attempt.refusal(&arguments)),
    }
}

fn head_or_empty_tree(root: &Path) -> Result<String, GitError> {
    match resolve_ref(root, "HEAD") {
        Ok(commit) => Ok(commit),
        Err(GitError::UnknownRef { .. }) => Ok(EMPTY_TREE.to_string()),
        Err(other) => Err(other),
    }
}

/// A base a diff can start from. The empty tree is not a commit, so
/// `rev-parse --verify <it>^{commit}` refuses it, and it is still the state a
/// repository before its first commit is diffed against.
fn resolve_base(root: &Path, base: &str) -> Result<String, GitError> {
    if base == EMPTY_TREE {
        return Ok(EMPTY_TREE.to_string());
    }
    resolve_ref(root, base)
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
    run_lossy(root, &arguments)
}

/// One git invocation whose answer is read as text even where some of it is not
/// utf-8. A diff carries the bytes of the files it is about, and a repository
/// with one latin-1 source file in it would otherwise leave weed unable to judge
/// anything else in the change. Nothing is lost that a rule reads: a rule
/// matches ascii shapes, and utf-8 never spells an ascii character with a byte
/// above 127, so a byte that gets replaced was never part of one. Every other
/// question weed asks git has a sha, a ref or a setting for an answer, and those
/// are still read strictly, bytes weed cannot read there are a refusal.
fn run_lossy(directory: &Path, arguments: &[&str]) -> Result<String, GitError> {
    let attempt = attempt(directory, arguments)?;
    if attempt.code == 0 {
        Ok(String::from_utf8_lossy(&attempt.stdout).into_owned())
    } else {
        Err(attempt.refusal(arguments))
    }
}

/// One git invocation that must have worked. Anything but success is a refusal
/// carrying git's own first line.
fn run(directory: &Path, arguments: &[&str]) -> Result<String, GitError> {
    let attempt = attempt(directory, arguments)?;
    if attempt.code == 0 {
        String::from_utf8(attempt.stdout).map_err(|error| GitError::Refused {
            command: arguments.join(" "),
            message: error.to_string(),
        })
    } else {
        Err(attempt.refusal(arguments))
    }
}

/// What one git invocation left behind, exit code and all. Some questions weed
/// asks, is this an ancestor, is this setting there, git answers by leaving
/// with a code, and those are answers rather than failures.
struct Attempt {
    code: i32,
    stdout: Vec<u8>,
    stderr: String,
}

impl Attempt {
    /// git's answer as one line of text. Every question asked this way has a
    /// sha, a ref or a setting for an answer; bytes weed cannot read as utf-8
    /// are a refusal rather than a guess.
    fn text(&self, arguments: &[&str]) -> Result<String, GitError> {
        String::from_utf8(self.stdout.clone())
            .map(|text| text.trim_end().to_string())
            .map_err(|error| GitError::Refused {
                command: arguments.join(" "),
                message: error.to_string(),
            })
    }

    fn refusal(&self, arguments: &[&str]) -> GitError {
        GitError::Refused {
            command: arguments.join(" "),
            message: first_line(&self.stderr),
        }
    }
}

/// One git invocation. Its config comes from the repository alone: a global
/// `quotepath` or an external diff driver must not change what weed judges.
fn attempt(directory: &Path, arguments: &[&str]) -> Result<Attempt, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory.as_os_str())
        .args(["-c", "core.quotepath=false"])
        .args(arguments.iter().map(OsStr::new))
        .output()
        .map_err(|error| GitError::Unavailable {
            message: error.to_string(),
        })?;

    Ok(Attempt {
        // A git killed by a signal leaves with no code of its own, and weed
        // must read that as a refusal rather than as any particular answer.
        code: output.status.code().unwrap_or(-1),
        stdout: output.stdout,
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("git said nothing")
        .to_string()
}
