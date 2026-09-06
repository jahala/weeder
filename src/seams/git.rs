//! The git seam: everything weeder knows about a repository, one function per
//! question. Every call is an argument array handed to `git`, never a shell
//! string, so a path or a ref that looks like a flag or a pipe is data.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::seams::fs;

/// The tree object every git repository shares for "nothing at all". A
/// repository before its first commit has no `HEAD`, and this is how git itself
/// spells the state such a repository is diffed against.
pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// The number of context lines weeder asks for. Three is git's own default, and
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
                "{path} is not inside a git repository, so there is no diff to judge. run weeder from a working tree."
            ),
            GitError::UnknownRef { reference } => write!(
                f,
                "this repository resolves no ref named {reference}. pass --base a ref it has, such as origin/main."
            ),
            GitError::Refused { command, message } => {
                write!(f, "git {command} failed: {message}.")
            }
            GitError::Unavailable { message } => {
                write!(f, "weeder could not run git: {message}. install git, or put it on PATH.")
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

/// One version of a file as a repository carries it, read once. What a rule
/// asks about a file, its lines, its weight, whether it carries lines at all,
/// is all answered from the same bytes rather than from a second read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blob {
    pub bytes: Vec<u8>,
}

impl Blob {
    /// What the file weighs.
    #[must_use]
    pub fn size(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// git's own test for a blob that is not text: a NUL byte. weeder judges
    /// lines, and these bytes carry none; G2 is the rule that reports one.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        self.bytes.contains(&0)
    }

    /// The file as text, or `None` where its bytes carry none. A byte that is
    /// not utf-8 is replaced rather than letting one latin-1 character make a
    /// file look like it is not there.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        (!self.is_binary()).then(|| String::from_utf8_lossy(&self.bytes).into_owned())
    }
}

/// A file at a ref: `None` where the ref does not carry the file.
pub fn file_at_ref(root: &Path, reference: &str, path: &str) -> Result<Option<Blob>, GitError> {
    Ok(files_at_ref(root, reference, &[path])?
        .into_iter()
        .flatten()
        .next())
}

/// Every one of these paths as a ref carries it, in the order they were asked
/// about, and `None` where the ref carries no such file.
///
/// git is asked twice however many files there are: once for what the ref
/// holds, once for the bytes behind it. A fifty-file change therefore costs two
/// processes rather than fifty, and starting a process is most of what reading
/// a small file costs. A ref this repository cannot resolve carries none of
/// them, which is the answer a repository before its first commit has.
pub fn files_at_ref(
    root: &Path,
    reference: &str,
    paths: &[&str],
) -> Result<Vec<Option<Blob>>, GitError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let listing = match run_lossy(root, &["ls-tree", "-r", "-z", "--full-tree", reference]) {
        Ok(listing) => listing,
        Err(GitError::Refused { .. }) => return Ok(vec![None; paths.len()]),
        Err(other) => return Err(other),
    };
    read_objects(root, paths, &tree_objects(&listing))
}

/// The same question of the index: what a commit would carry, for every path
/// at once. A path the index holds unmerged is carried at three stages and at
/// none of them is it what a commit would carry, so those are left out here as
/// they are everywhere else weeder reads the index.
pub fn files_in_index(root: &Path, paths: &[&str]) -> Result<Vec<Option<Blob>>, GitError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let listing = run_lossy(root, &["ls-files", "-s", "-z"])?;
    read_objects(root, paths, &index_objects(&listing))
}

/// What a `ls-tree -r -z` listing says each path is: `<mode> <type> <object>`,
/// a tab, and the path, one record per NUL. Only the blobs are kept, a gitlink
/// names a commit in another repository and there is no file here to read.
fn tree_objects(listing: &str) -> HashMap<&str, &str> {
    named(listing.split('\u{0}').filter_map(|record| {
        let (fields, path) = record.split_once('\t')?;
        let mut fields = fields.split_whitespace();
        let _mode = fields.next()?;
        let kind = fields.next()?;
        let object = fields.next()?;
        (kind == "blob").then_some((path, object))
    }))
}

/// The same of a `ls-files -s -z` listing: `<mode> <object> <stage>`, a tab,
/// and the path. Stage zero is the file a commit would carry; the stages of an
/// unmerged path are the sides of a merge nobody has finished.
fn index_objects(listing: &str) -> HashMap<&str, &str> {
    named(listing.split('\u{0}').filter_map(|record| {
        let (fields, path) = record.split_once('\t')?;
        let mut fields = fields.split_whitespace();
        let _mode = fields.next()?;
        let object = fields.next()?;
        let stage = fields.next()?;
        (stage == "0").then_some((path, object))
    }))
}

/// The object each path names, and none for a path that names two.
///
/// A listing is read as lossy text, and a repository may hold two paths whose
/// names differ only in bytes that are not utf-8: read that way they are one
/// path naming two files. Reading one file as another is the answer a judge
/// must never give, so neither is read, which is the answer weeder has always had
/// for a path it could not resolve.
fn named<'a>(records: impl Iterator<Item = (&'a str, &'a str)>) -> HashMap<&'a str, &'a str> {
    let mut found: HashMap<&str, &str> = HashMap::new();
    let mut ambiguous: Vec<&str> = Vec::new();
    for (path, object) in records {
        if found
            .insert(path, object)
            .is_some_and(|held| held != object)
        {
            ambiguous.push(path);
        }
    }
    for path in ambiguous {
        found.remove(path);
    }
    found
}

/// The bytes behind each path, in the order the paths were asked about. The
/// objects go to one `cat-file --batch`, which answers them in the order they
/// were written to it, so the answers pair back up by position.
fn read_objects(
    root: &Path,
    paths: &[&str],
    objects: &HashMap<&str, &str>,
) -> Result<Vec<Option<Blob>>, GitError> {
    let wanted: Vec<&str> = paths
        .iter()
        .filter_map(|path| objects.get(path).copied())
        .collect();
    let mut read = batch(root, &wanted)?.into_iter();
    paths
        .iter()
        .map(|path| match objects.contains_key(path) {
            false => Ok(None),
            true => read.next().ok_or_else(|| GitError::Refused {
                command: "cat-file --batch".to_string(),
                message: format!("git answered for fewer objects than weeder asked about, and {path} was one of them"),
            }),
        })
        .collect()
}

/// One `cat-file --batch` over these objects. It answers each one with a line
/// naming it, its type and its length, then that many bytes and a newline; an
/// object it cannot find it answers with the name and `missing`, which is not
/// a failure here, the file is simply not there to read.
fn batch(root: &Path, objects: &[&str]) -> Result<Vec<Option<Blob>>, GitError> {
    if objects.is_empty() {
        return Ok(Vec::new());
    }
    let mut asked = objects.join("\n");
    asked.push('\n');
    let attempt = attempt_writing(root, &["cat-file", "--batch"], asked.into_bytes())?;
    if attempt.code != 0 {
        return Err(attempt.refusal(&["cat-file", "--batch"]));
    }
    let mut answers = Vec::with_capacity(objects.len());
    let mut rest = attempt.stdout.as_slice();
    for object in objects {
        let (answer, remainder) = answer(rest, object)?;
        answers.push(answer);
        rest = remainder;
    }
    Ok(answers)
}

/// One object's answer, and what is left of the stream behind it.
fn answer<'a>(stream: &'a [u8], object: &str) -> Result<(Option<Blob>, &'a [u8]), GitError> {
    let unreadable = |what: &str| GitError::Refused {
        command: "cat-file --batch".to_string(),
        message: format!("weeder could not read git's answer for {object}: {what}"),
    };
    let end = stream
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or_else(|| unreadable("the line naming it never ends"))?;
    let header = String::from_utf8_lossy(&stream[..end]);
    let rest = &stream[end + 1..];
    let mut fields = header.split_whitespace();
    let _name = fields.next().ok_or_else(|| unreadable("it is empty"))?;
    let Some(kind) = fields.next() else {
        return Err(unreadable("it says neither a type nor `missing`"));
    };
    if kind == "missing" {
        return Ok((None, rest));
    }
    let Some(length) = fields.next() else {
        return Err(unreadable("it names no length"));
    };
    let size: usize = length
        .parse()
        .map_err(|_| unreadable("its length is not a number"))?;
    if rest.len() < size + 1 {
        return Err(unreadable("it is shorter than the length it names"));
    }
    Ok((
        Some(Blob {
            bytes: rest[..size].to_vec(),
        }),
        &rest[size + 1..],
    ))
}

/// A file in the working tree, or `None` where there is no such file.
pub fn file_in_tree(root: &Path, path: &str) -> Result<Option<Blob>, GitError> {
    let bytes = fs::read_bytes_if_present(&root.join(path)).map_err(|error| GitError::Refused {
        command: format!("show :{path}"),
        message: error.message,
    })?;
    Ok(bytes.map(|bytes| Blob { bytes }))
}

/// Every path the repository holds, as the index carries them: what a commit
/// built from this change would have in it. A rule that asks what a pattern
/// covers, or what an import points at, is asking about these.
pub fn tracked_paths(root: &Path) -> Result<Vec<String>, GitError> {
    let output = run_lossy(root, &["ls-files", "-z"])?;
    let mut paths: Vec<String> = output
        .split('\u{0}')
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
        .collect();
    paths.sort();
    paths.dedup();
    Ok(paths)
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
    // message with a byte weeder cannot read still has to be honoured.
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

/// This repository's own directory, where weeder keeps what must never reach a
/// commit and never travel with a clone.
pub fn git_dir(root: &Path) -> Result<PathBuf, GitError> {
    let path = run(root, &["rev-parse", "--absolute-git-dir"])?;
    Ok(PathBuf::from(path.trim_end()))
}

/// A setting in this repository's own configuration, or `None` where it carries
/// none. The repository's file alone is read: a setting a person keeps in their
/// global configuration is theirs, and weeder must not put it back as if it were
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

/// The settings weeder's own commits are made under. A machine with no identity
/// configured still has to be able to build the state a test runs in, and the
/// author of a commit nobody will ever see is weeder itself. Signing is off for
/// the same reason: a key that wants a passphrase would hold the run forever.
const AUTHORING: [&str; 3] = [
    "user.name=weeder bite",
    "user.email=bite@weeder.invalid",
    "commit.gpgsign=false",
];

/// What happened to a commit weeder tried to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// The change is in the worktree, as a commit of its own.
    Clean,
    /// git would not apply it, and this is what it said about that.
    Refused { message: String },
}

/// A second working tree of this repository, checked out at a commit, at a path
/// of weeder's choosing. `bite` needs one: it runs a test command over states the
/// caller's own checkout must never be made to hold.
pub fn add_worktree(root: &Path, at: &Path, commit: &str) -> Result<(), GitError> {
    let arguments = [
        OsStr::new("worktree"),
        OsStr::new("add"),
        OsStr::new("--detach"),
        at.as_os_str(),
        OsStr::new(commit),
    ];
    let added = attempt_os(root, &[], &arguments)?;
    if added.code == 0 {
        Ok(())
    } else {
        Err(GitError::Refused {
            command: format!("worktree add {}", at.display()),
            message: first_line(&added.stderr),
        })
    }
}

/// A worktree taken away again, with whatever the command that ran in it left
/// behind. A judge that scatters checkouts across a machine is one nobody runs
/// twice, so this is called on the way out of every path, verdict or refusal.
pub fn remove_worktree(root: &Path, at: &Path) -> Result<(), GitError> {
    let arguments = [
        OsStr::new("worktree"),
        OsStr::new("remove"),
        OsStr::new("--force"),
        at.as_os_str(),
    ];
    let removed = attempt_os(root, &[], &arguments)?;
    if removed.code == 0 {
        return Ok(());
    }
    // git keeps its own record of a worktree beside the repository. Where the
    // checkout could not be taken away, the record still can be, so the next
    // run is not judged by the leavings of this one.
    attempt(root, &["worktree", "prune"])?;
    Err(GitError::Refused {
        command: format!("worktree remove {}", at.display()),
        message: first_line(&removed.stderr),
    })
}

/// One commit's change, applied on top of whatever a worktree holds, as a
/// commit of its own. The hooks are told to stay out of it: a repository whose
/// hooks judge a commit must not get to judge the states weeder builds to ask a
/// question, and weeder's own guard is one of those hooks.
pub fn apply_commit(worktree: &Path, commit: &str, message: &str) -> Result<Applied, GitError> {
    let picked = attempt(worktree, &["cherry-pick", "--no-commit", commit])?;
    if picked.code != 0 {
        return Ok(Applied::Refused {
            message: first_line(&picked.stderr),
        });
    }
    let arguments = [
        OsStr::new("commit"),
        OsStr::new("--no-verify"),
        // A change already present in the base leaves nothing to commit, and
        // the state to run the tests in is the same either way.
        OsStr::new("--allow-empty"),
        OsStr::new("--message"),
        OsStr::new(message),
    ];
    let committed = attempt_os(worktree, &AUTHORING, &arguments)?;
    if committed.code == 0 {
        Ok(Applied::Clean)
    } else {
        Err(GitError::Refused {
            command: format!("commit {commit}"),
            message: first_line(&committed.stderr),
        })
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
/// with one latin-1 source file in it would otherwise leave weeder unable to judge
/// anything else in the change. Nothing is lost that a rule reads: a rule
/// matches ascii shapes, and utf-8 never spells an ascii character with a byte
/// above 127, so a byte that gets replaced was never part of one. Every other
/// question weeder asks git has a sha, a ref or a setting for an answer, and those
/// are still read strictly, bytes weeder cannot read there are a refusal.
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

/// What one git invocation left behind, exit code and all. Some questions weeder
/// asks, is this an ancestor, is this setting there, git answers by leaving
/// with a code, and those are answers rather than failures.
struct Attempt {
    code: i32,
    stdout: Vec<u8>,
    stderr: String,
}

impl Attempt {
    /// git's answer as one line of text. Every question asked this way has a
    /// sha, a ref or a setting for an answer; bytes weeder cannot read as utf-8
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
/// `quotepath` or an external diff driver must not change what weeder judges.
fn attempt(directory: &Path, arguments: &[&str]) -> Result<Attempt, GitError> {
    let arguments: Vec<&OsStr> = arguments.iter().map(|word| OsStr::new(*word)).collect();
    attempt_os(directory, &[], &arguments)
}

/// One git invocation with a list written to it. A child that answers with more
/// than a pipe holds would fill it and stop while weeder was still writing, and
/// weeder would stop waiting for a reader that is itself waiting, so the writing
/// goes out on a thread of its own and the reading happens here.
fn attempt_writing(
    directory: &Path,
    arguments: &[&str],
    input: Vec<u8>,
) -> Result<Attempt, GitError> {
    let unavailable = |error: std::io::Error| GitError::Unavailable {
        message: error.to_string(),
    };
    let mut child = Command::new("git")
        .arg("-C")
        .arg(directory.as_os_str())
        .args(["-c", "core.quotepath=false"])
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(unavailable)?;

    // A git that has answered everything it was asked leaves before the last of
    // the input reaches it, and the write fails with a closed pipe. That is an
    // answer arriving early rather than a failure, and what git wrote is read
    // below either way.
    let writing = child.stdin.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let _ = pipe.write_all(&input);
        })
    });
    let output = child.wait_with_output().map_err(unavailable)?;
    if let Some(writing) = writing {
        let _ = writing.join();
    }

    Ok(Attempt {
        code: output.status.code().unwrap_or(-1),
        stdout: output.stdout,
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// The same invocation, with settings of weeder's own in front of it and
/// arguments that are paths rather than words. A path this machine hands weeder
/// is not promised to be utf-8, and a worktree is named by one.
fn attempt_os(
    directory: &Path,
    settings: &[&str],
    arguments: &[&OsStr],
) -> Result<Attempt, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory.as_os_str())
        .args(["-c", "core.quotepath=false"])
        .args(settings.iter().flat_map(|setting| ["-c", setting]))
        .args(arguments)
        .output()
        .map_err(|error| GitError::Unavailable {
            message: error.to_string(),
        })?;

    Ok(Attempt {
        // A git killed by a signal leaves with no code of its own, and weeder
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
