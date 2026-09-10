//! The one way every weeder test builds its world.
//!
//! A test asks for a temp git repository, writes files into it, and runs the
//! built binary there. Nothing is mocked: the repository is real, the diff comes
//! from git, and the SARIF the tests read is what a caller would get.
//!
//! `fixture(rule, lang, case)` builds the repository from
//! `fixtures/adversarial/<RULE>/<lang>/<case>/`: `before/` is committed as HEAD,
//! `after/` replaces the tree, and files that `after/` does not carry are
//! deleted. `after/.weeder-commit` is the message of the commit being prepared ,
//! the harness hands it to weeder with `--message-file` and never copies it into
//! the tree, because a pre-commit gate has no commit to read a trailer from.
//! A rule that judges the tree rather than a diff has one state to read and not
//! two, so its fixture carries `before/` alone and the tree is left as `before/`
//! committed it.
//!
//! weeder does not carry what it refuses, so no file in this repository opens a
//! line with a conflict marker or holds a string shaped like a credential. Rust
//! tests build their markers with [`marker`] and its three shorthands; fixture
//! files spell theirs with the placeholders in `PLACEHOLDERS`, and spell a
//! credential with one from `CREDENTIALS`, or one a vendor published from
//! `PUBLISHED`, whose stamp and tail are written apart here and joined on the
//! way into the temp repository. The binary under test therefore reads exactly
//! what git writes into a file it could not merge, and exactly what an issuer
//! stamps.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::TempDir;

/// Fixed so two runs of the same fixture produce the same commit.
const AUTHOR_DATE: &str = "2026-09-05T09:00:00+00:00";
const AUTHOR_NAME: &str = "weeder fixtures";
const AUTHOR_EMAIL: &str = "fixtures@weeder.invalid";
/// The file `after/` uses to carry the pending commit message.
const COMMIT_MESSAGE_FILE: &str = ".weeder-commit";
/// The file `before/` uses to carry the date it was committed on, in any spelling
/// git reads, and never copied into the tree. A rule that asks git when a line
/// was written needs a history rather than a state, and this is how a fixture
/// states one without a test having to build the repository by hand.
const COMMIT_DATE_FILE: &str = ".weeder-date";
/// The name a fixture gives an ignore file. Written as `.gitignore`, the file
/// would govern the fixture's own directory and hide from git the very sources
/// the fixture carries next to it; under this name it is inert until copied.
pub const IGNORE_FILE_IN_FIXTURE: &str = "weeder.gitignore";
/// What that file is called once it is in the repository under test.
pub const IGNORE_FILE: &str = ".gitignore";
/// Where the harness keeps that message, outside the tree, until weeder is run.
const PENDING_MESSAGE_FILE: &str = "weeder-pending-message";

/// How many times a conflict marker repeats its character. git writes seven.
const MARKER_WIDTH: usize = 7;

/// What a fixture file writes where a conflict marker belongs, and the character
/// the harness expands it into.
const PLACEHOLDERS: &[(&str, char)] = &[
    ("{{weeder:ours}}", '<'),
    ("{{weeder:base}}", '|'),
    ("{{weeder:separator}}", '='),
    ("{{weeder:theirs}}", '>'),
];

/// What a fixture writes where a credential belongs: the placeholder, the stamp
/// an issuer puts on the front, and the opaque tail behind it. Neither half is a
/// credential on its own, so this repository carries none.
const CREDENTIALS: &[(&str, &str, &str)] = &[
    ("{{weeder:cloud-id}}", "AKIA", "3XAMPL3QRSTUVWXY"),
    (
        "{{weeder:forge-token}}",
        "ghp_",
        "0123456789abcdefghijklmnopqrstuvwx",
    ),
    (
        "{{weeder:forge-pat}}",
        "github_pat_",
        "11ABCDE0aBcDeFgHiJkLmNoPqRsTuVwXyZ",
    ),
    (
        "{{weeder:model-key}}",
        "sk-",
        "example0api0key0000000abcdefghij",
    ),
    (
        "{{weeder:chat-token}}",
        "xoxb-",
        "1111111111-2222222222-abcdefghijklmnopqrst",
    ),
    (
        "{{weeder:maps-key}}",
        "AIza",
        "SyA0example0key0value00000000000000",
    ),
    (
        "{{weeder:pipeline-token}}",
        "glpat-",
        "0123456789abcdefghij",
    ),
    (
        "{{weeder:registry-token}}",
        "npm_",
        "0123456789abcdefghijklmnopqrstuvwxyzAB",
    ),
    (
        "{{weeder:signed-token}}",
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
        ".eyJzdWIiOiIxMjM0NTY3ODkwIn0.dBjftJeZ4CVPmB92K27uhbUJU1p1r-wW1gFWFOEjXk",
    ),
    (
        "{{weeder:disordered}}",
        "",
        "9f3Kx2Qv7LmT4pR8sN1bY6cZ0dHwJ5eA",
    ),
];

/// The credentials vendors print in their own documentation, so a reader can
/// follow the page without one of their own: the name a fixture calls each by,
/// the stamp and the tail. These are quotations rather than keys, no issuer will
/// honour one, and weeder says so about them, which is what the fixtures here
/// prove. Written whole they would be the strings this repository refuses to
/// carry, so they are written apart and joined on the way into the temp
/// repository, the way every other credential is.
///
/// A fixture writes `{{weeder:example-<name>}}` for the example as its vendor
/// prints it, and `{{weeder:altered-<name>}}` for the same string with one
/// character changed, which is a credential again and nothing anyone published.
const PUBLISHED: &[(&str, &str, &str)] = &[
    ("cloud-id", "AKIA", "IOSFODNN7EXAMPLE"),
    (
        "cloud-secret",
        "wJalrXUtnFEMI",
        "/K7MDENG/bPxRfiCYEXAMPLEKEY",
    ),
    (
        "forge-token",
        "ghp_",
        "16C7e42F292c6912E7710c838347Ae178B4a",
    ),
    ("payment-key", "sk_test_", "4eC39HqLyjWDarjtT1zdp7dc"),
];

/// The names the fixtures write, in the order this harness holds them.
pub fn examples() -> Vec<&'static str> {
    PUBLISHED.iter().map(|(name, _, _)| *name).collect()
}

/// A published example, joined as its vendor prints it.
pub fn published_example(name: &str) -> String {
    let (_, stamp, tail) = example(name);
    format!("{stamp}{tail}")
}

/// The same example with one character changed: the same length, the same
/// shape, and not a string any vendor put in a manual.
pub fn altered_example(name: &str) -> String {
    let (_, stamp, tail) = example(name);
    let mut characters: Vec<char> = tail.chars().collect();
    let last = characters
        .last_mut()
        .expect("a published example has a tail");
    *last = stepped(*last);
    let tail: String = characters.into_iter().collect();
    format!("{stamp}{tail}")
}

fn example(name: &str) -> &'static (&'static str, &'static str, &'static str) {
    PUBLISHED
        .iter()
        .find(|(known, _, _)| *known == name)
        .unwrap_or_else(|| panic!("there is no published example called {name}"))
}

/// The next character of the alphabet the one handed over is written in, so a
/// value changed by one character is still a value of the same kind.
fn stepped(character: char) -> char {
    match character {
        '9' => '0',
        'z' => 'a',
        'Z' => 'A',
        digit if digit.is_ascii_digit() => (digit as u8 + 1) as char,
        letter if letter.is_ascii_alphabetic() => (letter as u8 + 1) as char,
        other => panic!("a published example ends in a letter or a digit, not {other:?}"),
    }
}

/// The placeholder a fixture writes where a private key block belongs.
const KEY_BLOCK: &str = "{{weeder:key-block}}";

/// The line a key file opens with, built from its parts. Written whole it would
/// be the very thing weeder refuses, so it is written in pieces and joined here.
fn key_block() -> String {
    let rule = "-".repeat(5);
    format!("{rule}BEGIN RSA PRIVATE {word}{rule}", word = "KEY")
}

/// The placeholder a fixture writes where bytes that are not text belong.
const BINARY_RUN: &str = "{{weeder:binary}}";

/// A run of bytes no diff can show. git's test for a file that is not text is a
/// NUL byte, and weeder asks the same question of the blob it reads, so the run
/// opens with one and carries the rest of the control bytes behind it. A fixture
/// that carried these bytes as bytes would be this repository carrying the very
/// blob G2 refuses, which is why they are written here and spelled with the
/// placeholder there.
fn binary_run() -> String {
    (0..=0x1f_u8).map(char::from).collect()
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

    /// A commit dated when the caller says, so a test about how old a line is
    /// can age one without waiting for the calendar. git reads the date from
    /// the environment, and the harness's own fixed date is overridden here.
    pub fn commit_dated(&self, message: &str, date: &str) {
        self.stage_all();
        let output = isolated(Command::new("git"))
            .arg("-C")
            .arg(self.root())
            .args(["-c", &format!("user.name={AUTHOR_NAME}")])
            .args(["-c", &format!("user.email={AUTHOR_EMAIL}")])
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date)
            .args(["commit", "-m", message])
            .output()
            .expect("git should be on PATH");
        assert!(
            output.status.success(),
            "the dated commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    pub fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"]).trim().to_string()
    }

    /// The message of the commit being prepared. It lives in the git dir, outside
    /// the tree, and every `weeder check` this repo runs passes it with `--message-file`.
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
    pub fn weeder(&self, arguments: &[&str]) -> Run {
        let output = self
            .weeder_command(arguments)
            .output()
            .expect("the weeder binary should run");
        Run {
            code: code(output.status),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    /// The built binary, run in this repository with something added to its
    /// environment. A repository whose docs cite a command needs that command
    /// on PATH, and the binary under test is the command the fixtures cite.
    pub fn weeder_with(&self, arguments: &[&str], environment: &[(&str, &str)]) -> Run {
        let mut command = self.weeder_command(arguments);
        for (name, value) in environment {
            command.env(name, value);
        }
        let output = command.output().expect("the weeder binary should run");
        Run {
            code: code(output.status),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    /// The built binary, run with an event written on its stdin. A harness hook
    /// is handed its event that way, so a test that asks what weeder does about
    /// one has to hand it over the same channel.
    pub fn weeder_reading(&self, arguments: &[&str], stdin: &str) -> Run {
        read_from(self.weeder_command(arguments), stdin)
    }

    /// The built binary, run with a real pseudo-terminal on stdout, so it
    /// answers the question a terminal asks rather than being told the answer.
    /// A pty is made with `openpty` here, which Windows has not: its own
    /// pseudo-console is another API and no crate this suite carries binds it.
    #[cfg(unix)]
    pub fn weeder_on_a_terminal(&self, arguments: &[&str]) -> Run {
        use std::io::Read;

        let terminal = Terminal::open();
        let mut child = self
            .weeder_command(arguments)
            .stdout(terminal.device())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the weeder binary should run");
        let stdout = terminal.read_to_end();
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            pipe.read_to_string(&mut stderr)
                .expect("stderr should read");
        }
        let status = child.wait().expect("the weeder binary should finish");
        Run {
            code: code(status),
            stdout,
            stderr,
        }
    }

    /// The binary in this repository; a pending message, where one was written,
    /// rides along as `--message-file` on every `check`.
    pub fn weeder_command(&self, arguments: &[&str]) -> Command {
        let mut command = weeder_command_in(self.root(), arguments);
        let pending = self.pending_message_path();
        if arguments.first() == Some(&"check") && pending.is_file() {
            command.arg("--message-file").arg(pending);
        }
        command
    }
}

/// The built binary, run in a directory the caller made, a repository or not.
pub fn weeder_in(directory: &Path, arguments: &[&str]) -> Run {
    let output = weeder_command_in(directory, arguments)
        .output()
        .expect("the weeder binary should run");
    Run {
        code: code(output.status),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

/// The built binary, run in a directory the caller made, with an event on its
/// stdin. A hook is handed its event that way even where no `Repo` built the
/// directory, a turn can end anywhere, including outside a repository.
pub fn weeder_reading_in(directory: &Path, arguments: &[&str], stdin: &str) -> Run {
    read_from(weeder_command_in(directory, arguments), stdin)
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
        .expect("the weeder binary should run");
    child
        .stdin
        .take()
        .expect("the child should have a stdin")
        .write_all(stdin.as_bytes())
        .expect("the event should be writable");
    let output = child
        .wait_with_output()
        .expect("the weeder binary should finish");
    Run {
        code: code(output.status),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

pub fn weeder_command_in(directory: &Path, arguments: &[&str]) -> Command {
    command_in(&binary(), directory, arguments)
}

/// A weeder binary a test put somewhere of its own, a copy, so the test can take
/// it away again and see what weeder says about a hook naming a binary that is gone.
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
    /// The SARIF log weeder wrote.
    pub fn log(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout).unwrap_or_else(|error| {
            panic!(
                "stdout should be a SARIF log, but it did not parse: {error}\n{}",
                self.stdout
            )
        })
    }

    /// Every result in the log, in the order weeder reported them.
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
    match dated(&repo) {
        Some(date) => repo.commit_dated("the state the change starts from", &date),
        None => repo.commit("the state the change starts from"),
    }

    let after = source.join("after");
    if !after.is_dir() {
        return repo;
    }
    empty_tree(repo.root());
    copy_tree(&after, repo.root());
    let pending = repo.root().join(COMMIT_MESSAGE_FILE);
    if pending.is_file() {
        let message = std::fs::read_to_string(&pending).expect("the pending message should read");
        std::fs::remove_file(&pending).expect("the pending message should not reach the tree");
        repo.pending_message(&message);
    }
    repo.stage_all();
    repo
}

/// A repository built from a phased fixture: `before/` committed as the base,
/// `test/` committed as the test phase, `after/` committed as the
/// implementation. Each stage is copied over the tree the stage before it
/// committed, because a phase adds to a repository rather than replacing it.
///
/// This is the history `weeder bite` judges: three commits, the middle one
/// carrying the tests alone, which is what a conductor that commits its test
/// phase separately leaves behind.
pub fn phased_fixture(rule: &str, lang: &str, case: &str) -> Repo {
    let source = fixture_root().join(rule).join(lang).join(case);
    assert!(
        source.is_dir(),
        "there is no fixture at {}",
        source.display()
    );

    let repo = Repo::init();
    for (stage, message) in [
        ("before", "the state the change starts from"),
        ("test", "the test phase"),
        ("after", "the implementation"),
    ] {
        copy_tree(&source.join(stage), repo.root());
        repo.commit(message);
    }
    repo
}

/// The date a fixture asked its state to be committed on, taken back out of the
/// tree so the repository under test carries only what the fixture wrote.
fn dated(repo: &Repo) -> Option<String> {
    let path = repo.root().join(COMMIT_DATE_FILE);
    if !path.is_file() {
        return None;
    }
    let date = std::fs::read_to_string(&path).expect("the fixture's date should read");
    std::fs::remove_file(&path).expect("the date should not reach the tree");
    Some(date.trim().to_string())
}

pub fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/adversarial")
}

/// One file of a fixture, read as the repository will hold it, placeholders
/// expanded, so a test can work out what it expects from the fixture itself.
pub fn fixture_file(rule: &str, lang: &str, case: &str, path: &str) -> String {
    let file = fixture_root().join(rule).join(lang).join(case).join(path);
    let contents = std::fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", file.display()));
    expand(&contents)
}

pub fn binary() -> PathBuf {
    assert_cmd::cargo::cargo_bin("weeder")
}

/// A PATH with the built binary's own directory in front of it, so a fixture
/// whose docs cite `weeder` is citing the binary under test.
pub fn path_with_weeder() -> String {
    let directory = binary()
        .parent()
        .expect("the built binary should sit in a directory")
        .display()
        .to_string();
    match std::env::var_os("PATH") {
        // The separator between two entries of a PATH is the platform's, a
        // colon on unix and a semicolon on Windows, so the list is built with
        // the one this machine reads rather than with either spelling.
        Some(path) => {
            let mut entries = vec![PathBuf::from(&directory)];
            entries.extend(std::env::split_paths(&path));
            std::env::join_paths(entries)
                .expect("a PATH the platform can hold")
                .to_string_lossy()
                .to_string()
        }
        None => directory,
    }
}

/// A file this machine will run, the way the platform spells that. On unix it is
/// the mode bits git and the kernel both read; on Windows there are none to
/// write, and a file that is there is a file that runs.
#[cfg(unix)]
pub fn make_runnable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("the file should be runnable");
}

/// A file this machine will run. Windows keeps no execute bit, so a file that is
/// there is already one, and there is nothing for this to write.
#[cfg(windows)]
pub fn make_runnable(_path: &Path) {}

/// A script a test owns, written into a directory it owns and made runnable.
/// A PATH built out of these is how a test sees which programs weeder reached
/// for: the program it would have run is right there, and it answers.
///
/// A `#!/bin/sh` file is a program where the kernel reads a shebang line, which
/// Windows does not do: there a program is an executable image, and nothing
/// starts a shell script. So the tests that put a program of their own on PATH
/// are unix tests, and each of them says so at its own gate.
#[cfg(unix)]
pub fn install_script(directory: &Path, name: &str, body: &str) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, body).expect("the script should be writable");
    make_runnable(&path);
    path
}

/// git, where a test has taken everything else off PATH. weeder asks git what the
/// tree holds before any rule runs, so a PATH without it is a scan that never
/// starts and proves nothing. It stands where [`install_script`] does: the
/// stand-in is a shell script, and Windows starts no script as a program.
#[cfg(unix)]
pub fn link_git(directory: &Path) {
    let found = which("git").expect("git should be on PATH");
    install_script(
        directory,
        "git",
        &format!(
            "#!/bin/sh\nexec {} \"$@\"\n",
            shell_word(&found.display().to_string())
        ),
    );
}

/// Where a program on PATH is, asked of the shell that owns the question. The
/// two halves are the same question: unix has a shell builtin for it, Windows
/// has a program, and neither exists on the other.
#[cfg(unix)]
pub fn which(program: &str) -> Option<PathBuf> {
    let found = Command::new("/usr/bin/env")
        .args(["sh", "-c", &format!("command -v {program}")])
        .output()
        .ok()?;
    found
        .status
        .success()
        .then(|| PathBuf::from(String::from_utf8_lossy(&found.stdout).trim()))
}

/// Where a program on PATH is, asked of the program Windows answers it with.
/// `where` prints one line per match, and the first is the one that would run.
#[cfg(windows)]
pub fn which(program: &str) -> Option<PathBuf> {
    let found = Command::new("where").arg(program).output().ok()?;
    if !found.status.success() {
        return None;
    }
    String::from_utf8_lossy(&found.stdout)
        .lines()
        .next()
        .map(|line| PathBuf::from(line.trim()))
}

/// A path as one word a shell cannot take apart.
pub fn shell_word(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
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
        let name = if entry.file_name() == OsStr::new(IGNORE_FILE_IN_FIXTURE) {
            OsStr::new(IGNORE_FILE).to_os_string()
        } else {
            entry.file_name()
        };
        let to = target.join(name);
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
    let text = PUBLISHED.iter().fold(text, |text, (name, _, _)| {
        text.replace(
            &format!("{{{{weeder:example-{name}}}}}"),
            &published_example(name),
        )
        .replace(
            &format!("{{{{weeder:altered-{name}}}}}"),
            &altered_example(name),
        )
    });
    text.replace(KEY_BLOCK, &key_block())
        .replace(BINARY_RUN, &binary_run())
}

/// The caller's git configuration, their hooks and their template directory stay
/// out of a fixture, so a test proves weeder rather than the machine it runs on.
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
    status.code().expect("weeder should leave with a code")
}

fn string(value: &serde_json::Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

/// A pseudo-terminal: the pair of file descriptors a terminal is made of. The
/// child writes to the device end, the test reads the control end. Windows has
/// no `openpty` and no file descriptors to pair, so this and the one method that
/// opens one are unix.
#[cfg(unix)]
struct Terminal {
    control: std::os::fd::OwnedFd,
    device: std::os::fd::OwnedFd,
}

/// The terminal's own methods, gated with it: Windows opens no pty here, so
/// there is nothing on that platform for them to be written about.
#[cfg(unix)]
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
        use std::io::Read;

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
