//! The latency budget.
//!
//! weed sits in front of a commit, so what it costs is what a person waits
//! before their editor comes back. Two numbers hold that: an ordinary change
//! answers in under 200 milliseconds, and the worst change anybody plausibly
//! stages, fifty files with a twenty mebibyte file among them, answers in under
//! two seconds. A gate slower than that is one a team turns off.
//!
//! The budget is measured on the release binary, because that is the one people
//! run. When these tests are themselves built in release, the binary beside them
//! is already it; otherwise the release binary is built first and measured. The
//! CI workflow runs the whole suite a second time on the release profile, so a
//! regression here fails the build rather than turning up as a gate somebody
//! quietly stops using.
//!
//! Every sample is printed. A machine under load can miss any single deadline,
//! so the assertion is on the median of a handful of runs, and the numbers
//! behind it are on screen when it fails.

// In a debug build the budget tests are absent, so what serves them is unused
// by design rather than by accident.
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use common::{fixture, weed_command_in, Repo};

/// What an ordinary change may take.
const ORDINARY: Duration = Duration::from_millis(200);

/// What the worst change anybody stages may take.
const WORST: Duration = Duration::from_secs(2);

/// How many files the worst case changes.
const FILES: usize = 50;

/// How large the one file among them that nobody should have staged is.
const HUGE: usize = 20 * 1024 * 1024;

/// How many times a case is timed. One run measures the machine's mood as much
/// as the binary; the middle of five measures the binary.
const SAMPLES: usize = 5;

/// The fixtures an ordinary change is measured on: one per language, each a
/// real diff weed has to read every rule over.
const ORDINARY_CASES: [(&str, &str); 4] = [("T1", "ts"), ("S1", "py"), ("G1", "rs"), ("X1", "go")];

/// The release binary, built if the test binary beside it is not one itself.
/// Nothing here measures a debug build: the numbers would be somebody else's.
/// The tests here run in threads of one process and would otherwise each start
/// a cargo of their own, so the build happens once and the rest read the answer.
fn release_binary() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(built_release)
}

fn built_release() -> PathBuf {
    let built = common::binary();
    let profile = built
        .parent()
        .expect("the built binary sits in a profile directory");
    if profile.file_name().is_some_and(|name| name == "release") {
        return built;
    }
    let target = profile
        .parent()
        .expect("a profile directory sits in a target directory");
    let release = target
        .join("release")
        .join(built.file_name().unwrap_or_default());
    build_release(target);
    assert!(
        release.is_file(),
        "the release build should leave a binary at {}",
        release.display()
    );
    release
}

/// `cargo build --release`, into the same target directory the test binary came
/// from, so a caller who moved that directory is still measuring their own tree.
fn build_release(target: &Path) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .env("CARGO_TARGET_DIR", target)
        .args(["build", "--release", "--bin", "weed"])
        .status()
        .expect("cargo should be on PATH");
    assert!(
        status.success(),
        "the release binary should build: {status}"
    );
}

/// One timed run of `weed check --staged` in a repository, with everything the
/// binary writes thrown away: the budget covers the judging, and a terminal on
/// the other end is not part of it.
fn once(repo: &Repo, binary: &Path) -> Duration {
    let mut command = common::command_in(binary, repo.root(), &["check", "--staged"]);
    let started = Instant::now();
    let output = command.output().expect("the weed binary should run");
    let taken = started.elapsed();
    assert!(
        output.status.code().is_some_and(|code| code != 3),
        "a run weed could not finish measures nothing: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    taken
}

/// The middle of `SAMPLES` timed runs, after one that is thrown away so the
/// first read of the tree is not the one being measured.
fn median(case: &str, repo: &Repo, binary: &Path) -> Duration {
    let _ = once(repo, binary);
    let mut taken: Vec<Duration> = (0..SAMPLES).map(|_| once(repo, binary)).collect();
    let samples: Vec<String> = taken
        .iter()
        .map(|each| format!("{}ms", each.as_millis()))
        .collect();
    println!("{case}: {}", samples.join(" "));
    taken.sort();
    taken[SAMPLES / 2]
}

/// A source file of `lines` lines, in the shape a repository holds one, with
/// `seed` written into it so no two of them are the same file.
fn source(seed: usize, lines: usize) -> String {
    let mut text = format!("export const seed{seed} = {seed};\n");
    for line in 0..lines {
        text.push_str(&format!(
            "export function step{seed}_{line}(value: number): number {{\n  return value + {line};\n}}\n"
        ));
    }
    text
}

/// The change nobody should be staging: fifty files touched, and one of them
/// twenty mebibytes of text a rule has to read to the end.
fn worst_case() -> Repo {
    let repo = Repo::init();
    for file in 0..FILES {
        repo.write(&format!("src/module{file}.ts"), &source(file, 20));
    }
    repo.commit("the state the change starts from");

    for file in 0..FILES - 1 {
        repo.write(&format!("src/module{file}.ts"), &source(file, 24));
    }
    let row = "the record carries a field, and the field carries a value\n";
    let huge = row.repeat(HUGE / row.len() + 1);
    assert!(
        huge.len() > HUGE,
        "the file has to be past the size it names"
    );
    repo.write(&format!("src/module{}.ts", FILES - 1), &huge);
    repo.stage_all();
    repo
}

// The budget is measured where it counts, on the release profile, which CI runs
// as its own job. A debug build times nothing: a wall-clock budget measured
// twice on two shared runners is twice the noise for the same answer.
#[cfg(not(debug_assertions))]
#[test]
fn an_ordinary_change_is_judged_in_under_two_hundred_milliseconds() {
    let binary = release_binary();
    for (rule, lang) in ORDINARY_CASES {
        let repo = fixture(rule, lang, "fire");
        let case = format!("{rule}/{lang}");
        let taken = median(&case, &repo, binary);
        assert!(
            taken < ORDINARY,
            "{case} took {}ms, and the budget is {}ms",
            taken.as_millis(),
            ORDINARY.as_millis()
        );
    }
}

// The budget is measured where it counts, on the release profile, which CI runs
// as its own job. A debug build times nothing: a wall-clock budget measured
// twice on two shared runners is twice the noise for the same answer.
#[cfg(not(debug_assertions))]
#[test]
fn the_worst_change_anybody_stages_is_judged_in_under_two_seconds() {
    let binary = release_binary();
    let repo = worst_case();

    let staged = common::git_in(repo.root(), &["diff", "--cached", "--name-only"]);
    assert_eq!(
        staged.stdout.lines().count(),
        FILES,
        "the worst case has to be {FILES} files, or the number below measures something smaller"
    );

    let taken = median("worst case", &repo, binary);
    assert!(
        taken < WORST,
        "the worst case took {}ms, and the budget is {}ms",
        taken.as_millis(),
        WORST.as_millis()
    );
}

/// The budget is only a budget if the build runs it. The release job in the CI
/// workflow runs the whole suite on the release profile, which is where these
/// two tests measure what they claim to.
#[test]
fn the_ci_workflow_runs_this_suite_on_the_release_profile() {
    let workflow = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let text = std::fs::read_to_string(&workflow).expect("the CI workflow should be readable");
    assert!(
        text.contains("cargo test --workspace --release"),
        "no job in {} runs the suite on the release profile",
        workflow.display()
    );
}

/// `weed_command_in` is the harness's way to the debug binary; this suite goes
/// through `command_in` with the release one instead, and this holds the two to
/// the same binary name so a rename cannot leave the budget measuring nothing.
#[test]
fn the_binary_measured_is_the_one_the_harness_names() {
    let harness = weed_command_in(Path::new(env!("CARGO_MANIFEST_DIR")), &[]);
    let named = Path::new(harness.get_program())
        .file_name()
        .expect("the harness names a binary");
    assert_eq!(
        release_binary().file_name(),
        Some(named),
        "the budget measures a different binary than the rest of the suite"
    );
}
