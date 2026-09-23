//! The law a change is judged under comes from the base of the range, never from
//! the change.
//!
//! A `weeder.toml` the change itself writes, commits or leaves lying in the
//! working tree is part of what is being judged, and a judge that took its law
//! from the accused would be no judge. So `check` reads the configuration at
//! `HEAD` for the working tree and for `--staged`, and at the base commit for
//! `--base` and for pre-push. A repository whose base carries no `weeder.toml` is
//! judged at the catalogue's own levels, and `--config` stays what it always was:
//! an explicit choice by whoever runs weeder.
//!
//! Every form is driven the way it runs in the field: `check` on a branch the
//! way CI runs it, `check --staged` the way a person does, `git commit` through
//! the installed guard, and the harness hook fed the event its harness writes.

mod common;

use common::Repo;
use serde_json::json;

/// A suite with two cases, the thing the change deletes.
const SUITE: &str = "const { add } = require(\"../src/add\");\n\n\
                     it(\"adds two numbers\", () => {\n  expect(add(1, 2)).toBe(3);\n});\n\n\
                     it(\"adds negatives\", () => {\n  expect(add(-1, -2)).toBe(-3);\n});\n";

const SOURCE: &str = "function add(a, b) {\n  return a + b;\n}\n\nmodule.exports = { add };\n";

/// A law that switches off the two rules that would report the change.
const SWITCHED_OFF: &str = "[rules]\nT1 = \"off\"\nC1 = \"off\"\n";

/// The python suite and module the staged forms judge.
const PY_SUITE: &str = "from src.parse import parse\n\n\n\
                        def test_parse_splits_on_commas():\n    assert parse(\"a,b\") == [\"a\", \"b\"]\n";
const PY_SKIPPED: &str = "import pytest\n\nfrom src.parse import parse\n\n\n\
                          @pytest.mark.skip\n\
                          def test_parse_splits_on_commas():\n    assert parse(\"a,b\") == [\"a\", \"b\"]\n";
const PY_SOURCE: &str = "def parse(line):\n    return line.split(\",\")\n";
const PY_STUB: &str = "def parse(line):\n    raise NotImplementedError\n";

/// What an agent leaves in the working tree and never stages.
const UNSTAGED_LAW: &str = "[rules]\nT3 = \"off\"\nS1 = \"off\"\n";

/// The words both pages use where they say which law a change is judged under.
const READ_FROM_THE_BASE: &str = "read from the base";

/// `main` carries the suite; the `pr` branch deletes it and, in the same
/// commit, writes the law that would let the deletion through.
fn branch_that_writes_its_own_law(base_law: Option<&str>) -> Repo {
    let repo = Repo::init();
    repo.write("src/add.js", SOURCE);
    repo.write("tests/add.test.js", SUITE);
    if let Some(law) = base_law {
        repo.write("weeder.toml", law);
    }
    repo.commit("the suite");
    repo.git(&["checkout", "-b", "pr"]);
    repo.remove("tests/add.test.js");
    repo.write("weeder.toml", SWITCHED_OFF);
    repo.commit("tidy the tests");
    repo
}

/// HEAD carries a suite and its module; the index carries a skip and a stub;
/// the working tree also holds a `weeder.toml` nobody staged that switches the
/// two rules off.
fn staged_skip_under_an_unstaged_law(repo: &Repo) {
    repo.write("src/parse.py", PY_SOURCE);
    repo.write("tests/test_parse.py", PY_SUITE);
    repo.commit("the parser and its test");
    repo.write("src/parse.py", PY_STUB);
    repo.write("tests/test_parse.py", PY_SKIPPED);
    repo.git(&["add", "src/parse.py", "tests/test_parse.py"]);
    repo.write("weeder.toml", UNSTAGED_LAW);
}

fn rules(run: &common::Run) -> Vec<String> {
    let mut rules: Vec<String> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.level == "error")
        .map(|finding| finding.rule)
        .collect();
    rules.sort();
    rules.dedup();
    rules
}

#[test]
fn a_branch_that_writes_a_law_switching_t1_off_is_judged_under_the_base_s_law() {
    let repo = branch_that_writes_its_own_law(None);

    let run = repo.weeder(&["check", "--base", "main", "--strict", "--format", "sarif"]);

    assert_eq!(
        run.code,
        2,
        "the deletion blocks, whatever the branch says about it: {}",
        run.output()
    );
    let blocked = rules(&run);
    assert!(
        blocked.contains(&"T1".to_string()),
        "T1 reports the deleted suite: {blocked:?}"
    );
    assert!(
        blocked.contains(&"C1".to_string()),
        "C1 judges the law being changed under the law it is changing: {blocked:?}"
    );
}

#[test]
fn a_law_the_base_carries_is_the_one_a_branch_is_judged_under() {
    // The base itself says T1 warns. The branch says it is off. The finding is
    // there, at the level the base gave it.
    let repo = branch_that_writes_its_own_law(Some("[rules]\nT1 = \"warn\"\nC1 = \"warn\"\n"));

    let run = repo.weeder(&["check", "--base", "main", "--strict", "--format", "sarif"]);

    assert_eq!(
        run.code,
        0,
        "the base's law warns and nothing blocks: {}",
        run.output()
    );
    let t1: Vec<common::Finding> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == "T1")
        .collect();
    assert_eq!(t1.len(), 1, "T1 still reports the deletion: {t1:#?}");
    assert_eq!(t1[0].level, "warning", "at the level the base gave it");
}

#[test]
fn a_staged_skip_under_an_unstaged_law_blocks() {
    let repo = Repo::init();
    staged_skip_under_an_unstaged_law(&repo);

    let run = repo.weeder(&["check", "--staged", "--format", "sarif"]);

    assert_eq!(run.code, 2, "the skip and the stub block: {}", run.output());
    let blocked = rules(&run);
    for rule in ["S1", "T3"] {
        assert!(
            blocked.contains(&rule.to_string()),
            "{rule} is on at HEAD, and HEAD is the law: {blocked:?}"
        );
    }
}

#[test]
fn the_working_tree_is_judged_under_head_s_law_too() {
    let repo = Repo::init();
    staged_skip_under_an_unstaged_law(&repo);

    let run = repo.weeder(&["check", "--format", "sarif"]);

    assert_eq!(run.code, 2, "the skip and the stub block: {}", run.output());
    let blocked = rules(&run);
    for rule in ["C1", "S1", "T3"] {
        assert!(
            blocked.contains(&rule.to_string()),
            "{rule} blocks, the untracked law included: {blocked:?}"
        );
    }
}

#[test]
fn git_commit_through_the_guard_refuses_a_staged_skip_under_an_unstaged_law() {
    let repo = Repo::init();
    repo.weeder(&["guard", "install"]);
    repo.commit("weeder guard installed");
    staged_skip_under_an_unstaged_law(&repo);
    let before = repo.head();

    let refused = repo.try_git(&["commit", "-m", "skip the flaky one"]);

    assert_ne!(
        refused.code,
        0,
        "git stops when the guard refuses: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(said.contains("T3"), "the hook names the skip:\n{said}");
    assert!(said.contains("S1"), "the hook names the stub:\n{said}");
    assert_eq!(repo.head(), before, "no commit was made");
}

#[test]
fn the_harness_hook_denies_a_commit_under_an_unstaged_law() {
    let repo = Repo::init();
    staged_skip_under_an_unstaged_law(&repo);
    let event = json!({
        "session_id": "a-session",
        "transcript_path": "/dev/null",
        "cwd": repo.root().display().to_string(),
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "git commit -m 'skip the flaky one'", "description": "commit"},
    })
    .to_string();

    let run = repo.weeder_reading(&["hook", "claude"], &event);

    assert_eq!(run.code, 2, "the hook denies the commit: {}", run.output());
    let said = run.output();
    assert!(said.contains("T3"), "the denial names the skip:\n{said}");
    assert!(said.contains("S1"), "the denial names the stub:\n{said}");
}

#[test]
fn a_base_with_no_law_is_judged_at_the_catalogue_s_levels() {
    // A repository with no weeder.toml at HEAD. The working tree writes one that
    // turns G1 off, and stages a conflict marker.
    let repo = Repo::init();
    repo.write("weeder.toml", "[rules]\nG1 = \"off\"\nC1 = \"off\"\n");
    repo.write("src/parser.ts", &common::conflicted_parser(None));
    repo.git(&["add", "src/parser.ts"]);

    let run = repo.weeder(&["check", "--staged", "--format", "sarif"]);

    assert_eq!(run.code, 2, "G1 blocks at its default: {}", run.output());
    assert!(rules(&run).contains(&"G1".to_string()), "{}", run.output());
}

#[test]
fn a_repository_before_its_first_commit_is_judged_at_the_catalogue_s_levels() {
    let repo = Repo::unborn();
    repo.write("weeder.toml", "[rules]\nG1 = \"off\"\nC1 = \"off\"\n");
    repo.write("src/parser.ts", &common::conflicted_parser(None));
    repo.stage_all();

    let run = repo.weeder(&["check", "--staged", "--format", "sarif"]);

    assert_eq!(
        run.code,
        2,
        "an unborn HEAD carries no law, so the defaults hold: {}",
        run.output()
    );
    assert!(rules(&run).contains(&"G1".to_string()), "{}", run.output());
}

#[test]
fn a_law_named_with_config_is_still_the_law_that_run_uses() {
    let repo = branch_that_writes_its_own_law(None);

    let run = repo.weeder(&[
        "check",
        "--base",
        "main",
        "--strict",
        "--format",
        "sarif",
        "--config",
        "weeder.toml",
    ]);

    assert_eq!(
        run.code,
        0,
        "whoever runs weeder may name the law outright: {}",
        run.output()
    );
    assert!(
        !run.findings().iter().any(|finding| finding.rule == "T1"),
        "T1 is off under the named law: {}",
        run.output()
    );
}

#[test]
fn a_base_whose_law_weeder_cannot_read_is_a_run_that_never_happened() {
    let repo = Repo::init();
    repo.write("weeder.toml", "[rules]\nG1 = \"loud\"\n");
    repo.commit("a law with a typo in it");
    repo.write("weeder.toml", "[rules]\nG1 = \"warn\"\n");

    let run = repo.weeder(&["check", "--format", "sarif"]);

    assert_eq!(run.code, 3, "{}", run.output());
    assert!(
        run.stderr.contains("G1") && run.stderr.contains("HEAD"),
        "the line names the key and the commit the law was read at: {}",
        run.stderr
    );
}

#[test]
fn skill_md_and_the_rules_page_say_the_law_is_read_from_the_base() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for page in ["SKILL.md", "docs/rules.md"] {
        let text = std::fs::read_to_string(root.join(page))
            .unwrap_or_else(|error| panic!("{page} should be readable: {error}"));
        let words = text.split_whitespace().collect::<Vec<&str>>().join(" ");
        assert!(
            words.contains(READ_FROM_THE_BASE),
            "{page} must say the law is {READ_FROM_THE_BASE} of the judged range"
        );
    }
}
