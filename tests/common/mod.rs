//! The one way every weed test builds its world.
//!
//! A test asks for a temp git repository, writes files into it, and runs the
//! built binary there. Nothing is mocked: the repository is real, the diff comes
//! from git, and the SARIF the tests read is what a caller would get.
//!
//! `fixture(rule, lang, case)` builds the repository from
//! `fixtures/adversarial/<RULE>/<lang>/<case>/`: `before/` is committed as HEAD,
//! `after/` replaces the tree, and files that `after/` does not carry are
//! deleted. `after/.weed-commit` is the message of the commit being prepared —
//! the harness hands it to weed with `--message-file` and never copies it into
//! the tree, because a pre-commit gate has no commit to read a trailer from.
//!
//! weed does not carry what it refuses, so no file in this repository opens a
//! line with a conflict marker or holds a string shaped like a credential. Rust
//! tests build their markers with [`marker`] and its three shorthands; fixture
//! files spell theirs with the placeholders in `PLACEHOLDERS`, and spell a
//! credential with one from `CREDENTIALS`, whose stamp and tail are written
//! apart here and joined on the way into the temp repository. The binary under
//! test therefore reads exactly what git writes into a file it could not merge,
//! and exactly what an issuer stamps.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::TempDir;

/// Fixed so two runs of the same fixture produce the same commit.
const AUTHOR_DATE: &str = "2026-09-05T09:00:00+00:00";
const AUTHOR_NAME: &str = "weed fixtures";
const AUTHOR_EMAIL: &str = "fixtures@weed.invalid";
/// The file `after/` uses to carry the pending commit message.
const COMMIT_MESSAGE_FILE: &str = ".weed-commit";
/// Where the harness keeps that message, outside the tree, until weed is run.
const PENDING_MESSAGE_FILE: &str = "weed-pending-message";

/// How many times a conflict marker repeats its character. git writes seven.
const MARKER_WIDTH: usize = 7;

/// What a fixture file writes where a conflict marker belongs, and the character
/// the harness expands it into.
const PLACEHOLDERS: &[(&str, char)] = &[
    ("{{weed:ours}}", '<'),
    ("{{weed:base}}", '|'),
    ("{{weed:separator}}", '='),
    ("{{weed:theirs}}", '>'),
];

/// What a fixture writes where a credential belongs: the placeholder, the stamp
/// an issuer puts on the front, and the opaque tail behind it. Neither half is a
/// credential on its own, so this repository carries none.
const CREDENTIALS: &[(&str, &str, &str)] = &[
    ("{{weed:cloud-id}}", "AKIA", "IOSFODNN7EXAMPLE"),
    (
        "{{weed:forge-token}}",
        "ghp_",
        "0123456789abcdefghijklmnopqrstuvwx",
    ),
    (
        "{{weed:forge-pat}}",
        "github_pat_",
        "11ABCDE0aBcDeFgHiJkLmNoPqRsTuVwXyZ",
    ),
    (
        "{{weed:model-key}}",
        "sk-",
        "example0api0key0000000abcdefghij",
    ),
    (
        "{{weed:chat-token}}",
        "xoxb-",
        "1111111111-2222222222-abcdefghijklmnopqrst",
    ),
    (
        "{{weed:maps-key}}",
        "AIza",
        "SyA0example0key0value00000000000000",
    ),
    ("{{weed:pipeline-token}}", "glpat-", "0123456789abcdefghij"),
    (
        "{{weed:registry-token}}",
        "npm_",
        "0123456789abcdefghijklmnopqrstuvwxyzAB",
    ),
    (
        "{{weed:signed-token}}",
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
        ".eyJzdWIiOiIxMjM0NTY3ODkwIn0.dBjftJeZ4CVPmB92K27uhbUJU1p1r-wW1gFWFOEjXk",
    ),
    (
        "{{weed:disordered}}",
        "",
        "9f3Kx2Qv7LmT4pR8sN1bY6cZ0dHwJ5eA",
    ),
];

/// The placeholder a fixture writes where a private key block belongs.
const KEY_BLOCK: &str = "{{weed:key-block}}";

/// The line a key file opens with, built from its parts. Written whole it would
/// be the very thing weed refuses, so it is written in pieces and joined here.
fn key_block() -> String {
    let rule = "-".repeat(5);
    format!("{rule}BEGIN RSA PRIVATE {word}{rule}", word = "KEY")
}

/// A conflict marker line as git writes it: seven of `character` at the start of
/// the line, then a space and `label` when the marker carries one.
pub fn marker(character: char, label: &str) -> String {
    let repeated: String = std::iter::repeat_n(character, MARKER_WIDTH).collect();
    if label.is_empty() {
        repeated
    } else {
        format!("{repeated} {label}")
    }
}

/// The marker that opens a conflict, above the side already in the file.
pub fn ours(label: &str) -> String {
    marker('<', label)
}

/// The marker between the two sides. It is also a setext rule and a table rule,
/// which is why a fixture needs one of these on its own.
pub fn separator() -> String {
    marker('=', "")
}

/// The marker that closes a conflict, below the side being merged in.
pub fn theirs(label: &str) -> String {
    marker('>', label)
}

/// The branch a fixture conflict is merging in. One name across the tests, so a
/// reader meets the same half-finished merge everywhere.
pub const BRANCH: &str = "feature/split-on-semicolons";

/// The file the check-face tests need most: a parser git could not merge, both
/// sides left in place. `note` is repeated above each marker when the test is
/// about suppressions, and left out otherwise.
pub fn conflicted_parser(note: Option<&str>) -> String {
    let note = note.map(|text| format!("  {text}\n")).unwrap_or_default();
    format!(
        "export function parse(input: string): string[] {{\n\
         {note}{opener}\n  return input.split(\",\");\n\
         {note}{separator}\n  return input.split(\";\");\n\
         {note}{closer}\n}}\n",
        opener = ours("HEAD"),
        separator = separator(),
        closer = theirs(BRANCH),
    )
}

pub struct Repo {
    directory: TempDir,
}

impl Repo {
    /// A git repository with one empty commit, so HEAD resolves.
    pub fn init() -> Repo {
        let repo = Repo::empty();
        repo.git(&["init", "--initial-branch=main"]);
        repo.git(&["commit", "--allow-empty", "-m", "the repository begins"]);
        repo
    }

    /// A bare repository: the thing a push goes to. It has no working tree and
    /// no first commit, so a test pushes one into it and reads back what arrived.
    pub fn bare() -> Repo {
        let repo = Repo::empty();
        repo.git(&["init", "--bare", "--initial-branch=main"]);
        repo
    }

    fn empty() -> Repo {
        Repo {
            directory: TempDir::new().expect("a temp directory for the repository"),
        }
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

    pub fn stage_all(&self) {
        self.git(&["add", "-A"]);
    }

    pub fn commit(&self, message: &str) {
        self.stage_all();
        self.git(&["commit", "-m", message]);
    }

    pub fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"]).trim().to_string()
    }

    /// The message of the commit being prepared. It lives in the git dir, outside
    /// the tree, and every `weed check` this repo runs passes it with `--message-file`.
    pub fn pending_message(&self, message: &str) {
        std::fs::write(self.pending_message_path(), message)
            .expect("the pending message should be writable");
    }

    fn pending_message_path(&self) -> PathBuf {
        let git_dir = self.git(&["rev-parse", "--git-dir"]);
        self.root().join(git_dir.trim()).join(PENDING_MESSAGE_FILE)
    }

    /// git, in this repository, with the caller's own configuration kept out.
    pub fn git(&self, arguments: &[&str]) -> String {
        let run = self.try_git(arguments);
        assert_eq!(
            run.code,
            0,
            "git {} failed in {}: {}{}",
            arguments.join(" "),
            self.root().display(),
            run.stdout,
            run.stderr
        );
        run.stdout
    }

    /// git, in this repository, where the test is asking whether it worked. A
    /// commit or a push a hook refuses leaves with a code, and that code is the
    /// answer the test came for.
    pub fn try_git(&self, arguments: &[&str]) -> Run {
        let output = isolated(Command::new("git"))
            .arg("-C")
            .arg(self.root())
            .args(["-c", &format!("user.name={AUTHOR_NAME}")])
            .args(["-c", &format!("user.email={AUTHOR_EMAIL}")])
            .args(arguments)
            .output()
            .expect("git should be on PATH");
        Run {
            code: code(output.status),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    /// The built binary, run in this repository with stdout on a pipe.
    pub fn weed(&self, arguments: &[&str]) -> Run {
        let output = self
            .weed_command(arguments)
            .output()
            .expect("the weed binary should run");
        Run {
            code: code(output.status),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    /// The built binary, run with an event written on its stdin. A harness hook
    /// is handed its event that way, so a test that asks what weed does about
    /// one has to hand it over the same channel.
    pub fn weed_reading(&self, arguments: &[&str], stdin: &str) -> Run {
        read_from(self.weed_command(arguments), stdin)
    }

    /// The built binary, run with a real pseudo-terminal on stdout, so it
    /// answers the question a terminal asks rather than being told the answer.
    pub fn weed_on_a_terminal(&self, arguments: &[&str]) -> Run {
        let terminal = Terminal::open();
        let mut child = self
            .weed_command(arguments)
            .stdout(terminal.device())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the weed binary should run");
        let stdout = terminal.read_to_end();
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            pipe.read_to_string(&mut stderr)
                .expect("stderr should read");
        }
        let status = child.wait().expect("the weed binary should finish");
        Run {
            code: code(status),
            stdout,
            stderr,
        }
    }

    /// The binary in this repository; a pending message, where one was written,
    /// rides along as `--message-file` on every `check`.
    pub fn weed_command(&self, arguments: &[&str]) -> Command {
        let mut command = weed_command_in(self.root(), arguments);
        let pending = self.pending_message_path();
        if arguments.first() == Some(&"check") && pending.is_file() {
            command.arg("--message-file").arg(pending);
        }
        command
    }
}

/// The built binary, run in a directory the caller made — a repository or not.
pub fn weed_in(directory: &Path, arguments: &[&str]) -> Run {
    let output = weed_command_in(directory, arguments)
        .output()
        .expect("the weed binary should run");
    Run {
        code: code(output.status),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

/// The built binary, run in a directory the caller made, with an event on its
/// stdin. A hook is handed its event that way even where no `Repo` built the
/// directory — a turn can end anywhere, including outside a repository.
pub fn weed_reading_in(directory: &Path, arguments: &[&str], stdin: &str) -> Run {
    read_from(weed_command_in(directory, arguments), stdin)
}

/// git, in a directory the caller made, with the caller's own configuration
/// kept out. A test that builds a second repository inside the first needs it.
pub fn git_in(directory: &Path, arguments: &[&str]) -> Run {
    let output = isolated(Command::new("git"))
        .arg("-C")
        .arg(directory)
        .args(["-c", &format!("user.name={AUTHOR_NAME}")])
        .args(["-c", &format!("user.email={AUTHOR_EMAIL}")])
        .args(arguments)
        .output()
        .expect("git should be on PATH");
    Run {
        code: code(output.status),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

/// A prepared command, run with `stdin` written into it and everything it wrote
/// read back.
fn read_from(mut command: Command, stdin: &str) -> Run {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the weed binary should run");
    child
        .stdin
        .take()
        .expect("the child should have a stdin")
        .write_all(stdin.as_bytes())
        .expect("the event should be writable");
    let output = child
        .wait_with_output()
        .expect("the weed binary should finish");
    Run {
        code: code(output.status),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

pub fn weed_command_in(directory: &Path, arguments: &[&str]) -> Command {
    command_in(&binary(), directory, arguments)
}

/// A weed binary a test put somewhere of its own — a copy, so the test can take
/// it away again and see what weed says about a hook naming a binary that is gone.
pub fn command_in(binary: &Path, directory: &Path, arguments: &[&str]) -> Command {
    let mut command = isolated(Command::new(binary));
    command
        .current_dir(directory)
        .args(arguments.iter().map(OsStr::new));
    command
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// The SARIF log weed wrote.
    pub fn log(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout).unwrap_or_else(|error| {
            panic!(
                "stdout should be a SARIF log, but it did not parse: {error}\n{}",
                self.stdout
            )
        })
    }

    /// Every result in the log, in the order weed reported them.
    pub fn findings(&self) -> Vec<Finding> {
        let log = self.log();
        let results = log["runs"][0]["results"]
            .as_array()
            .expect("a run carries a results array")
            .clone();
        results.iter().map(Finding::read).collect()
    }

    /// The paths the run reported a finding on, sorted and without repeats.
    pub fn paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .findings()
            .into_iter()
            .map(|finding| finding.path)
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }

    /// Everything the run wrote, whichever stream it chose. git hands a hook's
    /// output straight through, and which stream it lands on is git's business.
    pub fn output(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }

    pub fn stderr_lines(&self) -> Vec<&str> {
        self.stderr.lines().collect()
    }

    pub fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: String,
    pub level: String,
    pub path: String,
    pub line: Option<u64>,
    pub message: String,
    pub suppressed: bool,
}

impl Finding {
    fn read(result: &serde_json::Value) -> Finding {
        let location = &result["locations"][0]["physicalLocation"];
        Finding {
            rule: string(&result["ruleId"]),
            level: string(&result["level"]),
            path: string(&location["artifactLocation"]["uri"]),
            line: location["region"]["startLine"].as_u64(),
            message: string(&result["message"]["text"]),
            suppressed: result["suppressions"].is_array(),
        }
    }
}

/// A repository built from a fixture: `before/` as HEAD, `after/` as the tree.
pub fn fixture(rule: &str, lang: &str, case: &str) -> Repo {
    let source = fixture_root().join(rule).join(lang).join(case);
    assert!(
        source.is_dir(),
        "there is no fixture at {}",
        source.display()
    );

    let repo = Repo::init();
    copy_tree(&source.join("before"), repo.root());
    repo.commit("the state the change starts from");

    empty_tree(repo.root());
    copy_tree(&source.join("after"), repo.root());
    let pending = repo.root().join(COMMIT_MESSAGE_FILE);
    if pending.is_file() {
        let message = std::fs::read_to_string(&pending).expect("the pending message should read");
        std::fs::remove_file(&pending).expect("the pending message should not reach the tree");
        repo.pending_message(&message);
    }
    repo.stage_all();
    repo
}

pub fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/adversarial")
}

/// One file of a fixture, read as the repository will hold it — placeholders
/// expanded — so a test can work out what it expects from the fixture itself.
pub fn fixture_file(rule: &str, lang: &str, case: &str, path: &str) -> String {
    let file = fixture_root().join(rule).join(lang).join(case).join(path);
    let contents = std::fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", file.display()));
    expand(&contents)
}

pub fn binary() -> PathBuf {
    assert_cmd::cargo::cargo_bin("weed")
}

/// Every file in the working tree, gone. What `after/` carries comes back; what
/// it does not carry stays deleted, which is how a fixture removes a file.
fn empty_tree(root: &Path) {
    for entry in std::fs::read_dir(root).expect("the repository should be readable") {
        let entry = entry.expect("a repository entry should be readable");
        if entry.file_name() == OsStr::new(".git") {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path).expect("the directory should be removable");
        } else {
            std::fs::remove_file(&path).expect("the file should be removable");
        }
    }
}

fn copy_tree(source: &Path, target: &Path) {
    assert!(
        source.is_dir(),
        "the fixture has no {} directory",
        source.display()
    );
    for entry in std::fs::read_dir(source).expect("the fixture should be readable") {
        let entry = entry.expect("a fixture entry should be readable");
        let from = entry.path();
        let to = target.join(entry.file_name());
        if from.is_dir() {
            std::fs::create_dir_all(&to).expect("a fixture directory should be creatable");
            copy_tree(&from, &to);
        } else {
            let contents = std::fs::read_to_string(&from)
                .unwrap_or_else(|error| panic!("{} should be utf-8 text: {error}", from.display()));
            std::fs::write(&to, expand(&contents)).expect("a fixture file should be writable");
        }
    }
}

/// A fixture's text with its placeholders turned back into the bytes git writes
/// into a file it could not merge, and the strings an issuer stamps.
fn expand(contents: &str) -> String {
    let text = PLACEHOLDERS
        .iter()
        .fold(contents.to_string(), |text, (placeholder, character)| {
            text.replace(placeholder, &marker(*character, ""))
        });
    let text = CREDENTIALS
        .iter()
        .fold(text, |text, (placeholder, stamp, tail)| {
            text.replace(placeholder, &format!("{stamp}{tail}"))
        });
    text.replace(KEY_BLOCK, &key_block())
}

/// The caller's git configuration, their hooks and their template directory stay
/// out of a fixture, so a test proves weed rather than the machine it runs on.
fn isolated(mut command: Command) -> Command {
    command
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", AUTHOR_NAME)
        .env("GIT_AUTHOR_EMAIL", AUTHOR_EMAIL)
        .env("GIT_COMMITTER_NAME", AUTHOR_NAME)
        .env("GIT_COMMITTER_EMAIL", AUTHOR_EMAIL)
        .env("GIT_AUTHOR_DATE", AUTHOR_DATE)
        .env("GIT_COMMITTER_DATE", AUTHOR_DATE);
    command
}

fn code(status: std::process::ExitStatus) -> i32 {
    status.code().expect("weed should leave with a code")
}

fn string(value: &serde_json::Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

/// A pseudo-terminal: the pair of file descriptors a terminal is made of. The
/// child writes to the device end, the test reads the control end.
struct Terminal {
    control: std::os::fd::OwnedFd,
    device: std::os::fd::OwnedFd,
}

impl Terminal {
    fn open() -> Terminal {
        use std::os::fd::FromRawFd;

        let mut control = -1;
        let mut device = -1;
        // openpty writes the pair through these pointers and reports failure in
        // its return value; nothing else here reaches outside this frame.
        let opened = unsafe {
            libc::openpty(
                &mut control,
                &mut device,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(opened, 0, "the test needs a pseudo-terminal");
        Terminal {
            control: unsafe { std::os::fd::OwnedFd::from_raw_fd(control) },
            device: unsafe { std::os::fd::OwnedFd::from_raw_fd(device) },
        }
    }

    fn device(&self) -> Stdio {
        Stdio::from(
            self.device
                .try_clone()
                .expect("the terminal device should clone"),
        )
    }

    /// Everything the child wrote to the terminal, with the carriage returns a
    /// terminal adds taken back out.
    fn read_to_end(self) -> String {
        let Terminal { control, device } = self;
        // The last device handle must go, or the read never sees end of file.
        drop(device);
        let mut file = std::fs::File::from(control);
        let mut written = Vec::new();
        loop {
            let mut buffer = [0_u8; 4096];
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => written.extend_from_slice(&buffer[..read]),
                // A terminal whose device end is closed reports EIO on Linux and
                // end of file on macOS. Both mean the child is done writing.
                Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
                Err(error) => panic!("the terminal should read: {error}"),
            }
        }
        String::from_utf8_lossy(&written).replace('\r', "")
    }
}
