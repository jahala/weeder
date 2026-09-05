//! The last resort, proven rather than promised.
//!
//! Core returns its failures and never panics on input, and the property tests
//! in `core_hostile.rs` are what holds that up. A panic that gets past them is a
//! bug in weed — but a gate that dies mid-judgement must still fail closed, and
//! must say it was weed's fault and not the diff's. The hook in `main.rs` turns
//! any panic into exit 3 with one line.
//!
//! A debug build leaves a door open so this can be proven on the real binary:
//! `WEED_PANIC_FOR_TESTS` makes weed panic with the words it is given, before it
//! has read a flag or looked at a repository. `cfg(debug_assertions)` keeps the
//! door out of a release build.

mod common;

use common::{weed_in, Repo, Run};

/// The variable a debug build reads, and the words it panics with.
const ASKED: &str = "WEED_PANIC_FOR_TESTS";

fn panicking(repo: &Repo, arguments: &[&str], reason: &str) -> Run {
    let output = repo
        .weed_command(arguments)
        .env(ASKED, reason)
        .output()
        .expect("the weed binary should run");
    Run {
        code: output.status.code().unwrap_or_else(|| {
            panic!("weed left without a code of its own, so a signal killed it")
        }),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

#[test]
fn a_panic_leaves_with_exit_three_and_names_itself_a_bug() {
    let repo = Repo::init();
    let run = panicking(
        &repo,
        &["check"],
        "the seam handed core something it could not read",
    );

    assert_eq!(
        run.code, 3,
        "a panic is a run that never judged anything, so it leaves with 3 and not 101: {}",
        run.stderr
    );
    assert_eq!(
        run.stderr_lines().len(),
        1,
        "a run that could not run says why on one line: {}",
        run.stderr
    );
    let line = run.stderr.trim_end();
    assert!(
        line.starts_with("weed hit a bug and stopped rather than judge:"),
        "the line names weed as the one at fault: {line}"
    );
    assert!(
        line.ends_with("report it with the diff."),
        "the line ends with what to do next: {line}"
    );
    assert!(
        line.contains("the seam handed core something it could not read"),
        "the line carries what the panic said: {line}"
    );
    assert!(
        line.contains("src/main.rs:"),
        "the line carries where the panic happened: {line}"
    );
}

/// A panic weed cannot judge must not look like a judgement. Nothing may reach
/// stdout, where a SARIF log with no results would read as a clean run.
#[test]
fn a_panic_writes_no_log_a_reader_could_mistake_for_a_verdict() {
    let repo = Repo::init();
    let run = panicking(&repo, &["check", "--format", "sarif"], "a bug");
    assert_eq!(
        run.stdout, "",
        "a panic writes nothing to stdout, so nothing there can be read as clean"
    );
}

/// A panic message written across several lines would be several reasons as far
/// as a reader — or a hook parsing output a line at a time — can tell.
#[test]
fn a_panic_that_says_several_lines_still_leaves_one() {
    let repo = Repo::init();
    let run = panicking(
        &repo,
        &["check"],
        "the first thing\nthe second thing\n\nthe third thing",
    );
    assert_eq!(run.code, 3);
    assert_eq!(
        run.stderr_lines().len(),
        1,
        "the reason is folded back onto one line: {}",
        run.stderr
    );
    let line = run.stderr.trim_end();
    for part in ["the first thing", "the second thing", "the third thing"] {
        assert!(
            line.contains(part),
            "nothing the panic said is lost: {line}"
        );
    }
}

/// The hook catches a panic wherever it comes from, including before weed has
/// read a flag. `--version` is the shortest path through the binary there is.
#[test]
fn the_hook_is_in_place_before_the_flags_are_read() {
    let repo = Repo::init();
    let run = panicking(&repo, &["--version"], "a bug in the argument parser");
    assert_eq!(
        run.code, 3,
        "the hook is installed before anything that could panic runs"
    );
    assert!(run.stderr.contains("weed hit a bug"), "{}", run.stderr);
}

/// The door only opens when it is asked to. A weed nobody asked to fall over
/// judges the repository it was pointed at, exactly as it did before.
#[test]
fn weed_judges_as_usual_when_nobody_asks_it_to_fall_over() {
    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.stage_all();

    let run = repo.weed(&["check", "--format", "sarif"]);
    assert_eq!(run.code, 0, "a clean tree is clean: {}", run.stderr);
    assert!(
        run.stderr.is_empty(),
        "nothing stopped this run: {}",
        run.stderr
    );
    assert_eq!(
        run.log()["runs"][0]["invocations"][0]["executionSuccessful"],
        serde_json::Value::Bool(true)
    );
}

/// The door reads one variable and nothing else, so a repository that happens to
/// carry an odd environment cannot open it by accident.
#[test]
fn a_neighbouring_variable_does_not_open_the_door() {
    let repo = Repo::init();
    let output = repo
        .weed_command(&["check", "--format", "sarif"])
        .env("WEED_PANIC", "1")
        .env("WEED_PANIC_FOR_TEST", "1")
        .env("weed_panic_for_tests", "1")
        .output()
        .expect("the weed binary should run");
    assert_eq!(
        output.status.code(),
        Some(0),
        "only WEED_PANIC_FOR_TESTS opens the door: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The binary the rest of the suite runs is a debug build, which is what carries
/// the door. If that ever stops being true this test says so, rather than the
/// hook quietly going unproven.
#[test]
fn the_binary_under_test_is_the_one_that_carries_the_door() {
    let directory = tempfile::tempdir().expect("a temp directory");
    let output = weed_in(directory.path(), &["--version"]);
    assert_eq!(
        output.code, 0,
        "weed reports its version from anywhere: {}",
        output.stderr
    );

    let asked = std::process::Command::new(common::binary())
        .arg("--version")
        .env(ASKED, "a bug")
        .output()
        .expect("the weed binary should run");
    assert_eq!(
        asked.status.code(),
        Some(3),
        "the tests drive a debug build, so the door is there to prove the hook with"
    );
}
