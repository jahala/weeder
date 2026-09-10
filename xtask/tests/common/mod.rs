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
const AUTHOR_NAME: &str = "weeder measurements";
const AUTHOR_EMAIL: &str = "measurements@weeder.invalid";

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

    /// The commit the default branch is on, which is what a corpus pins.
    pub fn tip(&self) -> String {
        self.git(&["rev-parse", "main"]).trim().to_string()
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
        // `std::env::temp_dir` reads TMPDIR on unix and TEMP or TMP on Windows,
        // so all three are named and the run puts its scratch where the caller
        // can look for it on either platform.
        command.env("TMPDIR", scratch);
        command.env("TEMP", scratch);
        command.env("TMP", scratch);
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

/// Everything a measurement reads besides the history: the corpus it takes the
/// repositories from, the ledger it takes classifications from, the earlier run
/// it compares itself with, and the re-grade that decides how the verdict is
/// worded. A suite owns all four, so nothing a run says depends on what the
/// machine it runs on happens to hold.
pub struct Bench {
    directory: TempDir,
}

impl Default for Bench {
    fn default() -> Bench {
        Bench::new()
    }
}

impl Bench {
    pub fn new() -> Bench {
        let bench = Bench {
            directory: workspace(),
        };
        std::fs::write(bench.judgements(), "").expect("the ledger should be writable");
        std::fs::write(bench.first_run(), "").expect("the record should be writable");
        bench
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    pub fn out(&self) -> PathBuf {
        self.path().join("calibration.md")
    }

    pub fn corpus(&self) -> PathBuf {
        self.path().join("corpus.toml")
    }

    pub fn judgements(&self) -> PathBuf {
        self.path().join("judgements.toml")
    }

    pub fn first_run(&self) -> PathBuf {
        self.path().join("first-run.toml")
    }

    /// The re-grade. It is not written unless a suite writes one, which is the
    /// state the provisional wording is for.
    pub fn audit(&self) -> PathBuf {
        self.path().join("audit.md")
    }

    /// Every re-grade the bench carries, named the way the repository's own are
    /// read: the files called `audit*.md`, sorted. With none written the run is
    /// still pointed at `audit.md`, which is the state where no second party has
    /// read anything back.
    pub fn audits(&self) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = std::fs::read_dir(self.path())
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.starts_with("audit") && name.ends_with(".md"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        found.sort();
        if found.is_empty() {
            found.push(self.audit());
        }
        found
    }

    /// Write the re-grade, one row per sample.
    /// The bench's own re-grade declares itself blind: under the ruling of
    /// 2026-09-06 only a blind re-grade can lift the qualification, and a suite
    /// probing the wording is probing that one. A sighted or undeclared file is
    /// written with `write_audit_record` and a declaration of its own.
    pub fn write_audit(&self, samples: &[(&str, usize, usize)]) {
        std::fs::write(self.audit(), audit_text(Some("yes"), samples))
            .expect("the audit should be writable");
    }

    /// Write one named re-grade beside whatever others the bench holds, with the
    /// `Blind:` declaration it makes about itself, or none at all. The name is
    /// the suite's, so it can ask what the report reads off a file whose
    /// declaration and whose name disagree.
    pub fn write_audit_record(
        &self,
        name: &str,
        declaration: Option<&str>,
        samples: &[(&str, usize, usize)],
    ) -> PathBuf {
        self.write_audit_regrade(name, declaration, samples, &[])
    }

    /// The same, with the re-grade's own blocked commit sample under it: a
    /// repository, a commit and the class this auditor gave it. The verdict's
    /// floor is drawn from those rows, so a suite that wants to move the floor
    /// writes them.
    pub fn write_audit_regrade(
        &self,
        name: &str,
        declaration: Option<&str>,
        samples: &[(&str, usize, usize)],
        blocked: &[(&str, &str, &str)],
    ) -> PathBuf {
        let path = self.path().join(name);
        let mut text = audit_text(declaration, samples);
        if !blocked.is_empty() {
            text.push_str("\n## Blocked Commit Sample\n\n");
            text.push_str("| Repo | Commit | Auditor verdict | Reasoning |\n|---|---|---|---|\n");
            for (repo, sha, verdict) in blocked {
                text.push_str(&format!(
                    "| {repo} | `{sha}` | {verdict} | the auditor read the finding and the change |\n"
                ));
            }
        }
        std::fs::write(&path, text).expect("the audit should be writable");
        path
    }

    /// Write the record of an earlier run's block-level findings.
    pub fn write_first_run(&self, blocks: &[(&str, &str, &str, &str)]) {
        let mut text = String::new();
        for (repo, commit, rule, path) in blocks {
            text.push_str(&format!(
                "[[block]]\nrepo = \"{repo}\"\ncommit = \"{commit}\"\nrule = \"{rule}\"\npath = \"{path}\"\n\n"
            ));
        }
        std::fs::write(self.first_run(), text).expect("the record should be writable");
    }

    /// Name one repository, at one commit, as the whole corpus. A suite hands
    /// xtask its own corpus rather than narrowing the shipped one, so it judges
    /// the same history on a machine that has none of the garden checkouts.
    pub fn write_corpus(&self, name: &str, source: &Path, tip: &str) {
        // The source sits in a TOML basic string, where a backslash opens an
        // escape; a Windows path is written with forward slashes, which git
        // reads on every platform.
        let source = source.display().to_string().replace('\\', "/");
        std::fs::write(
            self.corpus(),
            format!("[[repo]]\nname = \"{name}\"\nsource = \"{source}\"\ntip = \"{tip}\"\n"),
        )
        .expect("the corpus should be writable");
    }

    fn arguments(&self, command: &str, extra: &[&str]) -> Vec<String> {
        let mut arguments = vec![
            command.to_string(),
            "--corpus".to_string(),
            self.corpus().display().to_string(),
        ];
        if command == "calibrate" {
            arguments.extend([
                "--out".to_string(),
                self.out().display().to_string(),
                "--judgements".to_string(),
                self.judgements().display().to_string(),
                "--first-run".to_string(),
                self.first_run().display().to_string(),
            ]);
            for audit in self.audits() {
                arguments.extend(["--audit".to_string(), audit.display().to_string()]);
            }
        }
        arguments.extend(extra.iter().map(|argument| (*argument).to_string()));
        arguments
    }

    /// Judge one repository's history at the commit it is on.
    pub fn calibrate(&self, name: &str, repo: &Repo, extra: &[&str]) -> Run {
        self.calibrate_at(name, repo, &repo.tip(), extra)
    }

    /// Judge one repository's history at a commit the caller picks, which is how
    /// a suite asks what a pin behind the branch tip does.
    pub fn calibrate_at(&self, name: &str, repo: &Repo, tip: &str, extra: &[&str]) -> Run {
        self.write_corpus(name, repo.root(), tip);
        let arguments = self.arguments("calibrate", extra);
        xtask(&arguments.iter().map(String::as_str).collect::<Vec<&str>>())
    }

    /// The same, with the directory the run may put scratch repositories in
    /// named by the caller.
    pub fn calibrate_under(&self, name: &str, repo: &Repo, scratch: &Path) -> Run {
        self.write_corpus(name, repo.root(), &repo.tip());
        let arguments = self.arguments("calibrate", &[]);
        xtask_under(
            &arguments.iter().map(String::as_str).collect::<Vec<&str>>(),
            scratch,
        )
    }

    /// The recall campaign over one repository's history, pinned at its tip.
    pub fn mutate(&self, name: &str, repo: &Repo, extra: &[&str]) -> Run {
        self.write_corpus(name, repo.root(), &repo.tip());
        let mut arguments = self.arguments("mutate", &[]);
        arguments.extend([
            "--out".to_string(),
            self.out().display().to_string(),
            "--json".to_string(),
            self.cases().display().to_string(),
            "--cases-dir".to_string(),
            self.path().join("kept").display().to_string(),
        ]);
        arguments.extend(extra.iter().map(|argument| (*argument).to_string()));
        // The clones the campaign fetches into land in the bench, so a suite
        // reads a history it built rather than one another run left behind.
        xtask_under(
            &arguments.iter().map(String::as_str).collect::<Vec<&str>>(),
            self.path(),
        )
    }

    /// Where the campaign writes what it planted.
    pub fn cases(&self) -> PathBuf {
        self.path().join("cases.json")
    }

    fn run_json(&self) -> serde_json::Value {
        let text = std::fs::read_to_string(self.cases())
            .expect("the campaign should write down what it planted");
        serde_json::from_str(&text).expect("the campaign should write json")
    }

    /// Every case the campaign planted.
    pub fn planted(&self) -> Vec<Planted> {
        self.run_json()["cases"]
            .as_array()
            .expect("the run should carry its cases")
            .iter()
            .map(|case| Planted {
                rule: text_at(case, "rule"),
                language: text_at(case, "language"),
                path: text_at(case, "path"),
                caught: case["caught"].as_bool().unwrap_or_default(),
            })
            .collect()
    }

    /// How many shapes the campaign wrote into a tree and could not read back,
    /// for one rule in one language.
    pub fn unplantable(&self, rule: &str, language: &str) -> usize {
        self.run_json()["unplantable"]
            .as_array()
            .expect("the run should carry what it could not plant")
            .iter()
            .filter(|held| text_at(held, "rule") == rule && text_at(held, "language") == language)
            .filter_map(|held| held["cases"].as_u64())
            .sum::<u64>() as usize
    }

    /// Count one repository's allowances at the commit it is pinned to.
    pub fn suppressions(&self, name: &str, repo: &Repo, tip: &str, extra: &[&str]) -> Run {
        self.write_corpus(name, repo.root(), tip);
        let arguments = self.arguments("suppressions", extra);
        xtask(&arguments.iter().map(String::as_str).collect::<Vec<&str>>())
    }

    pub fn report(&self) -> String {
        std::fs::read_to_string(self.out()).expect("the report should be written")
    }

    /// The report's paragraphs, blank line separated, the title first.
    pub fn paragraphs(&self) -> Vec<String> {
        self.report()
            .split("\n\n")
            .map(str::trim)
            .filter(|paragraph| !paragraph.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// The report's first sentence, which is the verdict.
    pub fn verdict(&self) -> String {
        self.report()
            .lines()
            .skip(1)
            .find(|line| !line.trim().is_empty())
            .unwrap_or_default()
            .to_string()
    }
}

/// One case a campaign planted, as it wrote it down.
pub struct Planted {
    pub rule: String,
    pub language: String,
    pub path: String,
    pub caught: bool,
}

fn text_at(held: &serde_json::Value, field: &str) -> String {
    held[field].as_str().unwrap_or_default().to_string()
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

/// One re-grade as a file: the declaration it makes about how it was taken, if
/// it makes one, and a row per sample.
fn audit_text(declaration: Option<&str>, samples: &[(&str, usize, usize)]) -> String {
    let mut text = String::from("# the re-grade\n\n");
    if let Some(declaration) = declaration {
        text.push_str(&format!("Blind: {declaration}\n\n"));
    }
    text.push_str("| Sample | Re-graded | Agreed |\n|---|---|---|\n");
    for (name, regraded, agreed) in samples {
        text.push_str(&format!("| {name} | {regraded} | {agreed} |\n"));
    }
    text
}

/// A repository with one blocking commit, so there is something to classify and
/// the verdict is a ship rather than a kill.
pub fn blocking_repo() -> Repo {
    let repo = Repo::init();
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.write("tests/unit.rs", &suite(3));
    repo.commit("the repository begins");

    repo.write("tests/unit.rs", &suite(2));
    repo.commit("one case fewer");
    repo
}

/// A repository whose last three commits each block, so a suite can classify
/// them apart and watch a share and a floor come out different.
pub fn three_blocking_commits() -> Repo {
    let repo = Repo::init();
    for file in ["alpha", "beta", "gamma"] {
        repo.write(&format!("tests/{file}.rs"), &suite(3));
    }
    repo.write("src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    repo.commit("the repository begins");
    for file in ["alpha", "beta", "gamma"] {
        repo.write(&format!("tests/{file}.rs"), &suite(2));
        repo.commit(&format!("one case fewer in {file}"));
    }
    repo
}

/// That repository with its one block classified, so the numbers clear the bar
/// and only the re-grade decides what the first paragraph says.
pub fn cleared() -> (Repo, Bench) {
    let repo = blocking_repo();
    let bench = Bench::new();
    std::fs::write(
        bench.judgements(),
        format!(
            "[[commit]]\nrepo = \"probe\"\nsha = \"{}\"\nclassification = \"true-positive\"\nreasoning = \"a case really did go\"\n",
            repo.tip()
        ),
    )
    .expect("the ledger should be writable");
    (repo, bench)
}
