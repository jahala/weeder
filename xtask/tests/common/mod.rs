//! A real repository for the measurements to read, built a commit at a time.
//!
//! Nothing here is a double. The suites want a history with known commits in it,
//! so they write files, commit them with git, and hand the directory to the same
//! `xtask` binary the alias runs.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// Fixed, so two runs of a suite build the same commits.
const AUTHOR_DATE: &str = "2026-09-05T09:00:00+00:00";
const AUTHOR_NAME: &str = "weed measurements";
const AUTHOR_EMAIL: &str = "measurements@weed.invalid";

pub struct Repo {
    directory: TempDir,
}

impl Repo {
    /// An empty repository on `main`, with no commit yet.
    pub fn init() -> Repo {
        let repo = Repo {
            directory: TempDir::new().expect("a temp directory for the repository"),
        };
        repo.git(&["init", "--initial-branch=main"]);
        repo
    }

    pub fn root(&self) -> &Path {
        self.directory.path()
    }

    pub fn write(&self, path: &str, contents: &str) {
        let target = self.root().join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).expect("a directory for the file");
        }
        std::fs::write(&target, contents).expect("the file should be writable");
    }

    pub fn remove(&self, path: &str) {
        std::fs::remove_file(self.root().join(path)).expect("the file should be removable");
    }

    /// Stage everything and commit it, returning the sha.
    pub fn commit(&self, message: &str) -> String {
        self.git(&["add", "-A"]);
        let output = isolated(Command::new("git"))
            .arg("-C")
            .arg(self.root())
            .args(["-c", &format!("user.name={AUTHOR_NAME}")])
            .args(["-c", &format!("user.email={AUTHOR_EMAIL}")])
            .env("GIT_AUTHOR_DATE", AUTHOR_DATE)
            .env("GIT_COMMITTER_DATE", AUTHOR_DATE)
            .args(["commit", "--quiet", "-m", message])
            .output()
            .expect("git should be on PATH");
        assert!(
            output.status.success(),
            "the commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        self.git(&["rev-parse", "HEAD"]).trim().to_string()
    }

    /// Everything a run could disturb, read the way the measurements read it.
    pub fn fingerprint(&self) -> String {
        format!(
            "{}{}{}",
            self.git(&["rev-parse", "HEAD"]),
            self.git(&["for-each-ref", "--format=%(objectname) %(refname)"]),
            self.git(&["status", "--porcelain=v1", "--untracked-files=all"]),
        )
    }

    pub fn git(&self, arguments: &[&str]) -> String {
        let output = isolated(Command::new("git"))
            .arg("-C")
            .arg(self.root())
            .args(arguments)
            .output()
            .expect("git should be on PATH");
        assert!(
            output.status.success(),
            "`git {}` failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).to_string()
    }
}

/// git with the machine's own configuration kept out, so a suite reads the same
/// on every machine.
fn isolated(mut command: Command) -> Command {
    command
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HOME", "/nonexistent");
    command
}

/// A directory the suite owns, for the files a run writes.
pub fn workspace() -> TempDir {
    TempDir::new().expect("a temp directory for the run")
}

pub fn binary() -> PathBuf {
    assert_cmd::cargo::cargo_bin("xtask")
}

/// Run the measurement binary and hand back what it did.
pub fn xtask(arguments: &[&str]) -> Run {
    run_xtask(arguments, None)
}

/// The same run, with the directory it may put scratch repositories in named by
/// the caller, so a suite can look in it afterwards.
pub fn xtask_under(arguments: &[&str], scratch: &Path) -> Run {
    run_xtask(arguments, Some(scratch))
}

fn run_xtask(arguments: &[&str], scratch: Option<&Path>) -> Run {
    let mut command = Command::new(binary());
    command.args(arguments);
    if let Some(scratch) = scratch {
        command.env("TMPDIR", scratch);
    }
    let output = command.output().expect("the xtask binary should be built");
    Run {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn succeeded(&self) -> &Run {
        assert_eq!(
            self.code, 0,
            "the run failed: {} {}",
            self.stdout, self.stderr
        );
        self
    }

    pub fn failed(&self) -> &Run {
        assert_ne!(
            self.code, 0,
            "the run was expected to fail: {}",
            self.stdout
        );
        self
    }
}

/// A test file with `cases` cases in it, each making one assertion.
pub fn suite(cases: usize) -> String {
    let mut source = String::new();
    for case in 0..cases {
        source.push_str(&format!(
            "#[test]\nfn case_{case}() {{\n    assert_eq!({case} + 1, {});\n}}\n\n",
            case + 1
        ));
    }
    source
}
