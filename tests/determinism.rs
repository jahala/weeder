//! The same answer every time.
//!
//! A gate that answers differently on two runs is a hole: a change lands
//! because the second run happened to sort a finding away. weeder's output is
//! byte-identical across runs for the same diff, tree and config, whatever the
//! locale, the time zone or the order a hash map happened to hand its keys back
//! in this process.
//!
//! Every fixture under `fixtures/adversarial/` is replayed three times through
//! the real binary, each run in a different locale and time zone, and the three
//! answers must match to the byte. Each run is its own process, so Rust's hash
//! maps are seeded differently every time, a map iterated into the log would
//! come apart here on its own.
//!
//! weeder writes no timestamp. If a later loop adds one, this is the test that
//! strips it, and the reason belongs in the comment that does the stripping.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{fixture, fixture_root, Repo, Run};

/// Three environments a run must not be able to tell apart. Turkish is here
/// because it is the locale that breaks case folding done the naive way: its
/// `i` does not upper-case to `I`. The time zones are far from UTC and from
/// each other, and Chatham is one of the few at a three-quarter-hour offset.
const ENVIRONMENTS: &[(&str, &str)] = &[
    ("C", "UTC"),
    ("en_US.UTF-8", "Asia/Kathmandu"),
    ("tr_TR.UTF-8", "Pacific/Chatham"),
];

/// A fixture as it is spelled on disk.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Case {
    rule: String,
    lang: String,
    case: String,
}

impl std::fmt::Display for Case {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.rule, self.lang, self.case)
    }
}

#[test]
fn every_fixture_answers_the_same_bytes_in_every_locale_and_time_zone() {
    let cases = cases();
    assert!(
        !cases.is_empty(),
        "there are no fixtures under {}, so this test proves nothing",
        fixture_root().display()
    );

    let mut reported = 0;
    for case in &cases {
        let repo = fixture(&case.rule, &case.lang, &case.case);
        reported += repo
            .weeder(&["check", "--format", "sarif"])
            .findings()
            .len();
        for arguments in [
            vec!["check", "--format", "sarif"],
            vec!["check", "--format", "table"],
            vec!["check", "--format", "sarif", "--strict"],
        ] {
            let replays: Vec<Run> = ENVIRONMENTS
                .iter()
                .map(|(locale, zone)| run(&repo, &arguments, locale, zone))
                .collect();
            let first = &replays[0];
            for (replay, (locale, zone)) in replays.iter().zip(ENVIRONMENTS) {
                assert_eq!(
                    replay.stdout,
                    first.stdout,
                    "{case} judged by `weeder {}` wrote different bytes under LANG={locale} TZ={zone}",
                    arguments.join(" ")
                );
                assert_eq!(
                    replay.stderr,
                    first.stderr,
                    "{case} judged by `weeder {}` complained differently under LANG={locale} TZ={zone}",
                    arguments.join(" ")
                );
                assert_eq!(
                    replay.code,
                    first.code,
                    "{case} judged by `weeder {}` left with a different code under LANG={locale} TZ={zone}",
                    arguments.join(" ")
                );
            }
        }
    }

    // Three empty logs are byte-identical too. The fixtures have to have found
    // something between them, or this test compares nothing three times.
    assert!(
        reported > 0,
        "no fixture reported a finding, so the replays compared empty logs"
    );
}

/// The catalogue is output too, and it is the one place a map over rule ids
/// would be easiest to reach for.
#[test]
fn the_rule_catalogue_prints_the_same_bytes_in_every_locale() {
    let repo = Repo::init();
    for arguments in [
        vec!["rules", "--format", "json"],
        vec!["rules", "--format", "table"],
    ] {
        let replays: Vec<Run> = ENVIRONMENTS
            .iter()
            .map(|(locale, zone)| run(&repo, &arguments, locale, zone))
            .collect();
        for replay in &replays {
            assert_eq!(
                replay.stdout,
                replays[0].stdout,
                "`weeder {}` wrote different bytes in another locale",
                arguments.join(" ")
            );
        }
    }
}

/// A log carrying a timestamp cannot be compared byte for byte, and a gate
/// whose output nobody can compare is a gate nobody can trust twice. weeder writes
/// none; this is the test that would have to start stripping one.
#[test]
fn the_log_carries_no_timestamp() {
    let repo = fixture("G1", "ts", "fire");
    let log = repo.weeder(&["check", "--format", "sarif"]).stdout;
    for word in [
        "startTimeUtc",
        "endTimeUtc",
        "\"timestamp\"",
        "utcTimeStamp",
    ] {
        assert!(
            !log.contains(word),
            "the log carries {word}, so two runs cannot be compared byte for byte: {log}"
        );
    }
}

/// Nothing in core may iterate a hash container into the output: two processes
/// seed their hashers differently, so the order it hands its keys back in is
/// not the same order twice. A container that is only ever looked up by key is
/// fine, and this test asks for that to be written down beside it.
#[test]
fn every_hash_container_in_core_says_why_its_order_never_reaches_the_output() {
    let mut undocumented = Vec::new();
    for file in rust_files(&core_root()) {
        let source = std::fs::read_to_string(&file).expect("a core source file should read");
        let lines: Vec<&str> = source.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !names_a_hash_container(line) {
                continue;
            }
            if !documented(&lines, index) {
                undocumented.push(format!("{}:{}: {}", file.display(), index + 1, line.trim()));
            }
        }
    }

    assert!(
        undocumented.is_empty(),
        "a HashMap or a HashSet in core needs a comment on it, or just above it, \
         saying what keeps its order out of the output, it hands its keys back \
         in a different order in every process:\n{}",
        undocumented.join("\n")
    );
}

/// The built binary in this repository, in a locale and a time zone of the
/// caller's choosing. Everything else about the two runs is the same.
fn run(repo: &Repo, arguments: &[&str], locale: &str, zone: &str) -> Run {
    let mut command = repo.weeder_command(arguments);
    let output = with_locale(&mut command, locale, zone)
        .output()
        .expect("the weeder binary should run");
    Run {
        code: output
            .status
            .code()
            .expect("weeder should leave with a code"),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

fn with_locale<'a>(command: &'a mut Command, locale: &str, zone: &str) -> &'a mut Command {
    command
        .env("LANG", locale)
        .env("LC_ALL", locale)
        .env("LANGUAGE", locale)
        .env("TZ", zone)
}

/// Every fixture on disk, so a rule landed after this loop is replayed without
/// anyone remembering to add it here.
fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for rule in directories(&fixture_root()) {
        for lang in directories(&rule) {
            for case in directories(&lang) {
                if !case.join("before").is_dir() || !case.join("after").is_dir() {
                    continue;
                }
                cases.push(Case {
                    rule: name(&rule),
                    lang: name(&lang),
                    case: name(&case),
                });
            }
        }
    }
    cases.sort();
    cases
}

fn directories(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut directories: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    directories.sort();
    directories
}

fn name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}

fn core_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core")
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return files;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}

/// A line that names a hash container in code. An import brings the name in and
/// iterates nothing, so it is the declarations and the uses that have to answer.
fn names_a_hash_container(line: &str) -> bool {
    let code = line.split("//").next().unwrap_or(line);
    let trimmed = code.trim_start();
    if trimmed.starts_with("use ") {
        return false;
    }
    code.contains("HashMap") || code.contains("HashSet")
}

/// Whether the use carries a comment that speaks about order, on the line
/// itself, or in the run of comment lines directly above it.
fn documented(lines: &[&str], index: usize) -> bool {
    let mut comment = String::new();
    if let Some((_, trailing)) = lines[index].split_once("//") {
        comment.push_str(trailing);
    }
    let mut above = index;
    while above > 0 {
        let candidate = lines[above - 1].trim_start();
        if !candidate.starts_with("//") {
            break;
        }
        comment.push(' ');
        comment.push_str(candidate);
        above -= 1;
    }
    comment.to_ascii_lowercase().contains("order")
}
