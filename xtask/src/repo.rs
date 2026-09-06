//! git, as the measurements need it: a read-only view of somebody else's
//! repository, a scratch clone to walk, and a fingerprint that says the source
//! is exactly as it was found.
//!
//! Nothing here writes to a source repository. A fetch runs `upload-pack` there
//! and takes objects away; every ref, every checkout and every judgement happens
//! in a directory under the system's temp dir that is removed at the end.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A git command that did not do what was asked, with what git said about it.
#[derive(Debug)]
pub struct GitError {
    pub command: String,
    pub message: String,
}

impl fmt::Display for GitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "`git {}` failed: {}", self.command, self.message)
    }
}

impl std::error::Error for GitError {}

/// Run git in a directory and hand back what it wrote on stdout, trailing
/// newline removed. A non-zero exit is an error carrying git's own words: a
/// measurement that guesses at what git meant is not a measurement.
pub fn git(directory: &Path, arguments: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .map_err(|error| GitError {
            command: arguments.join(" "),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(GitError {
            command: arguments.join(" "),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end_matches('\n')
        .to_string())
}

/// Whether git answers a question with a yes rather than an error, for the
/// questions whose answer is the exit code: does this ref exist, has this commit
/// a parent.
fn asks(directory: &Path, arguments: &[&str]) -> bool {
    git(directory, arguments).is_ok()
}

/// The branch a repository's history is judged on: the default branch its
/// upstream names, which is what a gate in CI would run against. The local
/// branch of the same name is not asked, a checkout on a machine can sit years
/// behind the branch it was cut from; where there is no upstream at all, the
/// conventional names are tried in turn, and then whatever HEAD is on.
pub fn default_ref(source: &Path) -> Result<String, GitError> {
    if let Ok(reference) = git(
        source,
        &["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"],
    ) {
        if !reference.is_empty() {
            return Ok(reference);
        }
    }
    for candidate in ["refs/heads/main", "refs/heads/master"] {
        if asks(source, &["rev-parse", "--verify", "--quiet", candidate]) {
            return Ok(candidate.to_string());
        }
    }
    git(source, &["rev-parse", "--symbolic-full-name", "HEAD"])
}

/// Everything about a repository that a run could disturb: where HEAD points,
/// every ref and what it points at, the worktrees it has, and what the working
/// tree and index hold that the last commit does not. Taken before the
/// measurement and again after, it is the evidence that reading a repository
/// left it alone.
pub fn fingerprint(source: &Path) -> Result<String, GitError> {
    let head = git(source, &["rev-parse", "HEAD"])?;
    let symbolic = git(source, &["rev-parse", "--symbolic-full-name", "HEAD"])?;
    let refs = git(
        source,
        &["for-each-ref", "--format=%(objectname) %(refname)"],
    )?;
    let worktrees = git(source, &["worktree", "list", "--porcelain"])?;
    let worktree = git(
        source,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    Ok(format!(
        "{symbolic} {head}\n{refs}\n{worktrees}\n{worktree}\n"
    ))
}

/// A scratch copy of one repository's default branch, under the system's temp
/// dir, that the walk checks out commit by commit and that is removed when the
/// run ends.
#[derive(Debug)]
pub struct Scratch {
    path: PathBuf,
}

/// The one branch a scratch repository holds, so the walk never has to ask which
/// of the source's branches it landed on.
const BRANCH: &str = "refs/heads/calibration";

impl Scratch {
    /// Fetch a source repository's default branch into a fresh repository under
    /// `parent`. `git fetch` from a path reads the source and writes nothing to
    /// it, which is why the walk never adds a worktree over there.
    pub fn fetch(parent: &Path, name: &str, source: &Path) -> Result<Scratch, GitError> {
        let path = parent.join(name);
        std::fs::create_dir_all(&path).map_err(|error| GitError {
            command: format!("init {}", path.display()),
            message: error.to_string(),
        })?;
        git(&path, &["init", "--quiet"])?;
        // A judgement reads a file at a ref, and the walk checks a commit out,
        // so the scratch repository holds the objects rather than borrowing
        // them: nothing it does can reach back into the source.
        let reference = default_ref(source)?;
        let refspec = format!("+{reference}:{BRANCH}");
        git(
            &path,
            &[
                "fetch",
                "--quiet",
                "--no-tags",
                &source.display().to_string(),
                &refspec,
            ],
        )?;
        Ok(Scratch { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The window of commits to judge, newest first.
    ///
    /// A merge commit is left out: it carries no change of its own, and the
    /// commits it brings are in the same window, so judging it as well would
    /// weigh one change twice.
    pub fn window(&self, limit: usize) -> Result<Vec<String>, GitError> {
        let limit = limit.to_string();
        Ok(git(
            &self.path,
            &["rev-list", "--no-merges", "-n", &limit, BRANCH],
        )?
        .split_whitespace()
        .map(str::to_string)
        .collect())
    }

    /// Every commit on the branch, oldest first, with the date it was committed.
    /// The suppression rate is a rate over time, so it needs the whole branch
    /// and not the window calibration judges.
    pub fn history(&self) -> Result<Vec<Commit>, GitError> {
        let log = git(
            &self.path,
            &["log", "--reverse", "--format=%H%x1f%cI%x1f%B%x1e", BRANCH],
        )?;
        Ok(log
            .split('\u{1e}')
            .map(str::trim_start)
            .filter(|record| !record.is_empty())
            .filter_map(|record| {
                let mut fields = record.splitn(3, '\u{1f}');
                Some(Commit {
                    sha: fields.next()?.to_string(),
                    committed: fields.next()?.to_string(),
                    message: fields.next()?.to_string(),
                })
            })
            .collect())
    }

    /// The first parent of a commit, or nothing where it has none. A root commit
    /// has no parent to be judged against, and this measurement judges commits
    /// against their parents.
    pub fn parent(&self, sha: &str) -> Result<Option<String>, GitError> {
        let argument = format!("{sha}^");
        match git(&self.path, &["rev-parse", "--verify", "--quiet", &argument]) {
            Ok(parent) if !parent.is_empty() => Ok(Some(parent)),
            _ => Ok(None),
        }
    }

    /// The subject line of a commit, for the table a reader scans.
    pub fn subject(&self, sha: &str) -> Result<String, GitError> {
        git(&self.path, &["log", "-1", "--format=%s", sha])
    }

    /// Put the working tree at a commit, which is the state `weed check --base
    /// <parent>` judges.
    pub fn checkout(&self, sha: &str) -> Result<(), GitError> {
        git(&self.path, &["checkout", "--quiet", "--detach", sha])?;
        Ok(())
    }

    /// Whether the checkout carries a `weed.toml`, and what it says. A
    /// repository that states its own law would be judged under it, and the
    /// report has to say so rather than let a reader assume the defaults.
    pub fn config_file(&self) -> Option<String> {
        std::fs::read_to_string(self.path.join("weed.toml")).ok()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// One commit as the suppression rate reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub sha: String,
    /// The commit date, ISO 8601 with its offset, as git wrote it.
    pub committed: String,
    pub message: String,
}
