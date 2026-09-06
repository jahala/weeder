//! The latency budget.
//!
//! weeder sits in front of a commit, so what it costs is what a person waits
//! before their editor comes back. Two numbers hold that: an ordinary change
//! answers in under 200 milliseconds, and the worst change anybody plausibly
//! stages, fifty files with a twenty mebibyte file among them, answers in under
//! two seconds. A gate slower than that is one a team turns off.
//!
//! The two-second budget is what a GitHub-hosted runner has to hold, and a
//! runner that shares its machine is the best part of twice as slow as the one
//! a worker builds on. So on a machine of its own the worst case is held to a
//! second, and the second between that and the budget is the room the budget
//! has on the slower machine.
//!
//! The budget is measured on the release binary, because that is the one people
//! run. When these tests are themselves built in release, the binary beside them
//! is already it; otherwise the release binary is built first and measured. The
//! CI workflow runs the whole suite a second time on the release profile, and
//! `scripts/check/run.sh` runs this file there too, so the number a loop cites
//! is the number CI holds and not a debug build's.
//!
//! Every sample is printed. A machine under load can miss any single deadline,
//! so the assertion is on the median of a handful of runs, and the numbers
//! behind it are on screen when it fails.
//!
//! What makes the worst case the worst case is the file nobody should have
//! staged. Past `core::change::READABLE` weeder stops reading a file as code, and
//! the two tests at the bottom hold the two halves of that: the face reads such
//! a file once, into the mask every rule then reads, so turning the rules on
//! adds no second pass over its bytes; and a rule that catches something in a
//! file that size still catches it.

// In a debug build the budget tests are absent, so what serves them is unused
// by design rather than by accident.
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use weeder::core::catalogue;
use weeder::core::change::READABLE;

use common::{altered_example, fixture, weeder_command_in, Repo};

/// What an ordinary change may take.
const ORDINARY: Duration = Duration::from_millis(200);

/// What the worst change anybody stages may take. This is the budget weeder
/// promises, and a GitHub-hosted runner is the machine that has to hold it.
const WORST: Duration = Duration::from_secs(2);

/// What it may take on a machine of its own. A hosted runner shares its
/// hardware with whatever else is on it and is the best part of twice as slow
/// as the machine a worker builds on, so the second between this and the budget
/// above is the room the budget is given there.
const HEADROOM: Duration = Duration::from_secs(1);

/// How much of its reading a run may add when the rules are turned on.
///
/// A file too large to read as code is read once, by the face, into the mask
/// every rule is handed. Turning the rules on therefore adds the judging and
/// no second pass over the bytes. Half again is the room that leaves; a run
/// that read the file once per rule that asks would be several times the
/// reading this case is made of, and would not fit under it.
const READ_AGAIN: f32 = 1.5;

/// How many files the worst case changes.
const FILES: usize = 50;

/// How large the one file among them that nobody should have staged is.
const HUGE: usize = 20 * 1024 * 1024;

/// How many times a case is timed. One run measures the machine's mood as much
/// as the binary; the middle of five measures the binary.
/// How many runs the warm-up may take before the timed samples are taken
/// regardless: a machine that never settles is measured as it is.
const WARM_UP_BOUND: usize = 8;
const SAMPLES: usize = 5;

/// The fixtures an ordinary change is measured on: one per language, each a
/// real diff weeder has to read every rule over.
const ORDINARY_CASES: [(&str, &str); 4] = [("T1", "ts"), ("S1", "py"), ("G1", "rs"), ("X1", "go")];

/// What a run judges: everything the catalogue carries, or nothing at all.
const JUDGING: [&str; 2] = ["check", "--staged"];

/// The config a repository writes to turn every rule off, kept out of the
/// change so the run being timed is the same change either way.
const NOTHING_JUDGED: &str = "nothing.toml";

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
        .args(["build", "--release", "--bin", "weeder"])
        .status()
        .expect("cargo should be on PATH");
    assert!(
        status.success(),
        "the release binary should build: {status}"
    );
}

/// One timed run of `weeder check --staged` in a repository, with everything the
/// binary writes thrown away: the budget covers the judging, and a terminal on
/// the other end is not part of it.
fn once(repo: &Repo, binary: &Path, arguments: &[&str]) -> Duration {
    let mut command = common::command_in(binary, repo.root(), arguments);
    let started = Instant::now();
    let output = command.output().expect("the weeder binary should run");
    let taken = started.elapsed();
    assert!(
        output.status.code().is_some_and(|code| code != 3),
        "a run weeder could not finish measures nothing: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    taken
}

/// The middle of `SAMPLES` timed runs, after one that is thrown away so the
/// first read of the tree is not the one being measured.
fn median(case: &str, repo: &Repo, binary: &Path, arguments: &[&str]) -> Duration {
    // The budget is the binary's cost at steady state, not the machine's first
    // minute. A shared runner has shown three runs at twice the settled time
    // before it settles, so the warm-up runs until three in a row agree within
    // a tenth, up to a bound, and only then are the timed samples taken.
    let mut warm: Vec<Duration> = Vec::new();
    for _ in 0..WARM_UP_BOUND {
        warm.push(once(repo, binary, arguments));
        if let [a, b, c] = warm[warm.len().saturating_sub(3)..] {
            let (low, high) = (
                [a, b, c].iter().min().copied(),
                [a, b, c].iter().max().copied(),
            );
            if let (Some(low), Some(high)) = (low, high) {
                if warm.len() >= 3 && high.as_millis() * 10 <= low.as_millis() * 11 {
                    break;
                }
            }
        }
    }
    let warmed: Vec<String> = warm
        .iter()
        .map(|each| format!("{}ms", each.as_millis()))
        .collect();
    println!("{case} warm-up: {}", warmed.join(" "));
    let mut taken: Vec<Duration> = (0..SAMPLES)
        .map(|_| once(repo, binary, arguments))
        .collect();
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

/// A file of `bytes` or a little more, in a language weeder reads, holding
/// nothing any rule has anything to say about.
fn filler(bytes: usize) -> String {
    let row = "the record carries a field, and the field carries a value\n";
    let text = row.repeat(bytes / row.len() + 1);
    assert!(
        text.len() > bytes,
        "the file has to be past the size it names"
    );
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
    repo.write(&format!("src/module{}.ts", FILES - 1), &filler(HUGE));
    repo.stage_all();
    repo
}

/// A repository whose history already carries the file nobody should have
/// committed, with one line added to it. What such a run costs is what reading
/// the file costs: there is one line for the rules to judge and twenty
/// mebibytes for the face to read.
fn one_line_on_an_unreadable_file() -> Repo {
    let repo = Repo::init();
    repo.write("src/ledger.ts", &filler(HUGE));
    repo.write(NOTHING_JUDGED, &nothing_judged());
    repo.commit("the ledger nobody should have committed");

    repo.write(
        "src/ledger.ts",
        &format!("{}export const appended = 1;\n", filler(HUGE)),
    );
    repo.stage_all();
    repo
}

/// A `weeder.toml` that turns off every rule the catalogue carries, so a run
/// under it does the reading and none of the judging.
fn nothing_judged() -> String {
    let mut text = String::from("[rules]\n");
    for rule in catalogue::rules() {
        text.push_str(&format!("{} = \"off\"\n", rule.id));
    }
    text
}

// The budget is measured where it counts, on the release profile, which CI runs
// as its own job and which the evidence runner asks for by name. A debug build
// times nothing: the numbers would be somebody else's.
#[cfg(not(debug_assertions))]
#[test]
fn an_ordinary_change_is_judged_in_under_two_hundred_milliseconds() {
    let binary = release_binary();
    for (rule, lang) in ORDINARY_CASES {
        let repo = fixture(rule, lang, "fire");
        let case = format!("{rule}/{lang}");
        let taken = median(&case, &repo, binary, &JUDGING);
        assert!(
            taken < ORDINARY,
            "{case} took {}ms, and the budget is {}ms",
            taken.as_millis(),
            ORDINARY.as_millis()
        );
    }
}

// The budget is measured where it counts, on the release profile, which CI runs
// as its own job and which the evidence runner asks for by name. A debug build
// times nothing: the numbers would be somebody else's.
#[cfg(not(debug_assertions))]
#[test]
fn the_worst_change_anybody_stages_leaves_the_budget_room_to_spare() {
    let binary = release_binary();
    let repo = worst_case();

    let staged = common::git_in(repo.root(), &["diff", "--cached", "--name-only"]);
    assert_eq!(
        staged.stdout.lines().count(),
        FILES,
        "the worst case has to be {FILES} files, or the number below measures something smaller"
    );

    let (allowed, machine) = budget();
    let taken = median("worst case", &repo, binary, &JUDGING);
    assert!(
        taken < allowed,
        "the worst case took {}ms on {machine}, where it has {}ms",
        taken.as_millis(),
        allowed.as_millis()
    );
}

/// What the worst case may take here, and the machine that says so.
///
/// The claim being held up is a second on a machine of its own, which is what
/// leaves the two-second budget its room on a machine that is shared. Holding a
/// shared runner to the tighter of the two would fail the build for the very
/// slowness the budget was written to absorb, so there it is the budget itself
/// that is measured. `CI` is the variable a hosted runner sets, and it is the
/// only thing a wall clock has to tell one machine from the other by.
fn budget() -> (Duration, &'static str) {
    match std::env::var_os("CI").is_some() {
        true => (WORST, "a runner sharing its machine"),
        false => (HEADROOM, "a machine of its own"),
    }
}

/// A file past the size weeder reads as code costs one pass over its bytes and no
/// more. The face makes that pass; every rule reads what it produced. So a run
/// with the whole catalogue on may cost more than a run with none of it on,
/// what it costs is the judging, and it must not cost the reading twice.
#[cfg(not(debug_assertions))]
#[test]
fn turning_the_rules_on_does_not_read_an_unreadable_file_again() {
    let binary = release_binary();
    let repo = one_line_on_an_unreadable_file();

    let reading = median(
        "unreadable, nothing judged",
        &repo,
        binary,
        &["check", "--staged", "--config", NOTHING_JUDGED],
    );
    let judging = median("unreadable, every rule", &repo, binary, &JUDGING);

    assert!(
        judging < reading.mul_f32(READ_AGAIN),
        "reading the file took {}ms and judging it took {}ms, which is more than the {READ_AGAIN} of it the rules may add: a rule is reading the file again",
        reading.as_millis(),
        judging.as_millis()
    );
}

/// The other half of the cap: a file too large to parse is still a file whose
/// lines are read, so what a rule catches in one it catches in the other. The
/// two files here carry the same code; one of them carries a great deal of
/// nothing behind it, which is the only difference between them.
#[test]
fn nothing_a_rule_catches_in_an_unreadable_file_is_lost() {
    let repo = Repo::init();
    repo.write("src/small.ts", "export const opening = 1;\n");
    repo.write("src/big.ts", "export const opening = 1;\n");
    repo.commit("the state the change starts from");

    let judged = judged_lines();
    let padded = format!("{judged}{}", filler(READABLE as usize));
    assert!(
        padded.len() as u64 > READABLE,
        "the padded file has to be past the size weeder stops reading as code"
    );
    repo.write("src/small.ts", &judged);
    repo.write("src/big.ts", &padded);
    repo.stage_all();

    let run = repo.weeder(&["check", "--staged", "--format", "sarif"]);
    let small = reported(&run, "src/small.ts");
    let big = reported(&run, "src/big.ts");
    assert!(
        !small.is_empty(),
        "the lines this test plants have to be lines a rule catches, and none of them was: {}",
        run.stdout
    );
    assert_eq!(
        big, small,
        "the same code was judged differently for being at the end of a file too large to parse"
    );
}

/// A handful of lines a rule catches: a work marker in production code, and a
/// credential written into it. The credential is one its issuer published, so
/// this repository carries no key of its own and weeder still reads the shape.
fn judged_lines() -> String {
    format!(
        "export const opening = 1;\n\
         // TODO: read the separator off the header rather than guessing it\n\
         export const parse = (line: string) => line.split(\",\");\n\
         export const token = \"{}\";\n",
        altered_example("cloud-id")
    )
}

/// The rules a run reported against one path.
fn reported(run: &common::Run, path: &str) -> BTreeSet<String> {
    run.findings()
        .into_iter()
        .filter(|finding| finding.path == path)
        .map(|finding| finding.rule)
        .collect()
}

/// The budget is only a budget if the build runs it. The release job in the CI
/// workflow runs the whole suite on the release profile, which is where these
/// tests measure what they claim to.
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

/// And the evidence runner asks for the same profile, so the loop that cites
/// this file cites a measurement rather than a suite that compiled the
/// measurements out.
#[test]
fn the_evidence_runner_asks_for_the_release_profile() {
    let runner = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/check/run.sh");
    let text = std::fs::read_to_string(&runner).expect("the evidence runner should be readable");
    assert!(
        text.contains("cargo test --release --test speed"),
        "{} runs this file on a profile that measures nothing",
        runner.display()
    );
}

/// `weeder_command_in` is the harness's way to the debug binary; this suite goes
/// through `command_in` with the release one instead, and this holds the two to
/// the same binary name so a rename cannot leave the budget measuring nothing.
#[test]
fn the_binary_measured_is_the_one_the_harness_names() {
    let harness = weeder_command_in(Path::new(env!("CARGO_MANIFEST_DIR")), &[]);
    let named = Path::new(harness.get_program())
        .file_name()
        .expect("the harness names a binary");
    assert_eq!(
        release_binary().file_name(),
        Some(named),
        "the budget measures a different binary than the rest of the suite"
    );
}
