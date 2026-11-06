//! A release that renames or removes a `weeder.toml` key leaves every
//! repository that used it with a base law the new binary cannot read (#100).
//! The law comes from the base, so the commit that moves the key is judged
//! under the old law, and refusing it as a run that never happened locked the
//! repository out: no hook and no CI run would let the repair land.
//!
//! A change that rewrites `weeder.toml` and nothing else is judged instead as
//! if the base carried no law, at the catalogue's own levels, and says so in a
//! note. It still never supplies its own law, C1 still blocks it, and a person
//! still signs it. Any other change over that base is still a run that never
//! happened, and the refusal names the way through.
//!
//! Every form that reads the law from a base is driven the way it runs in the
//! field: `git commit` and `git push` through the installed guard, `check
//! --base --strict` the way CI runs it, and the harness hook fed its event.

mod common;

use common::Repo;
use serde_json::json;

/// A law an older weeder read and this one refuses: a key it no longer has.
const OLD_LAW: &str = "[scope]\nspecimens = [\"fixtures/adversarial\"]\n";

/// The same intent, written the way this weeder reads it.
const MIGRATED: &str = "[scope]\nexclude = [\"fixtures/adversarial/\"]\n";

const SOURCE: &str = "export function parse(input: string): string[] {\n  \
                      return input.split(\";\");\n}\n";
const WIDENED: &str = "export function parse(input: string): string[] {\n  \
                       return input.split(/[,;]/);\n}\n";

/// The id of the note a run over an unreadable base law writes.
const NOTICE: &str = "LAW";

/// What the refusal says about the way through: the migration, landed before
/// anything else and with nothing else in it.
const OF_ITS_OWN: &str = "migration first, as a commit of its own";

const SIGNED: &str =
    "Move the law to the new key\n\nWeeder-allow: C1 the release renamed specimens to exclude\n";
const UNSIGNED: &str = "Move the law to the new key\n";

/// A repository guarded by weeder whose `HEAD` carries the old law. The law was
/// committed past the hooks, because no weeder that refuses the key would have
/// let it land; an older one did, and that is the history being repaired.
fn guarded_over_an_old_law() -> Repo {
    let repo = Repo::init();
    repo.write("src/parser.ts", SOURCE);
    repo.commit("the parser");
    repo.weeder(&["guard", "install"]);
    repo.write("weeder.toml", OLD_LAW);
    repo.stage_all();
    repo.git(&[
        "commit",
        "--no-verify",
        "-m",
        "the law an older weeder read",
    ]);
    repo
}

/// `main` carries the old law; the `pr` branch is checked out on top of it.
fn branch_over_an_old_law() -> Repo {
    let repo = Repo::init();
    repo.write("src/parser.ts", SOURCE);
    repo.write("weeder.toml", OLD_LAW);
    repo.commit("the parser, under the law an older weeder read");
    repo.git(&["checkout", "-b", "pr"]);
    repo
}

fn blocking(run: &common::Run) -> Vec<String> {
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
fn a_signed_commit_that_only_migrates_the_law_lands_through_the_guard() {
    let repo = guarded_over_an_old_law();
    let before = repo.head();
    repo.write("weeder.toml", MIGRATED);
    repo.git(&["add", "weeder.toml"]);

    let landed = repo.try_git(&["commit", "-m", SIGNED]);

    assert_eq!(
        landed.code,
        0,
        "the migration lands once a person signs it: {}",
        landed.output()
    );
    assert_ne!(repo.head(), before, "the commit was made");
    let said = landed.output();
    assert!(
        said.contains(NOTICE) && said.contains("catalogue"),
        "the run says the base's law could not be read and which levels judged it:\n{said}"
    );
}

#[test]
fn an_unsigned_commit_that_only_migrates_the_law_is_refused_on_c1() {
    let repo = guarded_over_an_old_law();
    let before = repo.head();
    repo.write("weeder.toml", MIGRATED);
    repo.git(&["add", "weeder.toml"]);

    let refused = repo.try_git(&["commit", "-m", UNSIGNED]);

    assert_ne!(
        refused.code,
        0,
        "git stops the commit: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(
        said.contains("C1"),
        "C1 blocks the law being changed:\n{said}"
    );
    assert!(
        !said.contains("is not valid"),
        "the refusal is a judgement, not a run that never happened:\n{said}"
    );
    assert_eq!(repo.head(), before, "no commit was made");
}

#[test]
fn a_commit_that_migrates_the_law_and_touches_anything_else_cannot_be_judged() {
    let repo = guarded_over_an_old_law();
    let before = repo.head();
    repo.write("weeder.toml", MIGRATED);
    repo.write("src/parser.ts", WIDENED);
    repo.git(&["add", "weeder.toml", "src/parser.ts"]);

    let refused = repo.try_git(&["commit", "-m", SIGNED]);

    assert_ne!(
        refused.code,
        0,
        "git stops the commit: {}",
        refused.output()
    );
    let said = refused.output();
    assert!(
        said.contains("specimens"),
        "the refusal names the key this weeder cannot read:\n{said}"
    );
    assert!(
        said.contains(OF_ITS_OWN),
        "the refusal names the migration as a commit of its own:\n{said}"
    );
    assert_eq!(repo.head(), before, "no commit was made");
}

#[test]
fn ci_judges_a_branch_that_only_migrates_the_law_and_blocks_it_on_c1() {
    let repo = branch_over_an_old_law();
    repo.write("weeder.toml", MIGRATED);
    repo.commit(SIGNED);

    let run = repo.weeder(&["check", "--base", "main", "--strict", "--format", "sarif"]);

    assert_eq!(
        run.code,
        2,
        "the migration is judged, and C1 blocks it until a person signs at the pull request: {}",
        run.output()
    );
    assert_eq!(blocking(&run), vec!["C1".to_string()], "{}", run.output());
    let notes: Vec<common::Finding> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == NOTICE)
        .collect();
    assert_eq!(notes.len(), 1, "one note says why: {}", run.output());
    assert_eq!(notes[0].level, "note");
}

#[test]
fn ci_cannot_judge_a_branch_that_migrates_the_law_and_changes_anything_else() {
    let repo = branch_over_an_old_law();
    repo.write("weeder.toml", MIGRATED);
    repo.write("src/parser.ts", WIDENED);
    repo.commit(SIGNED);

    let run = repo.weeder(&["check", "--base", "main", "--strict", "--format", "sarif"]);

    assert_eq!(run.code, 3, "{}", run.output());
    assert!(
        run.stderr.contains("specimens") && run.stderr.contains(OF_ITS_OWN),
        "the line names the key and the way through: {}",
        run.stderr
    );
}

#[test]
fn every_commit_of_a_range_must_touch_only_the_law() {
    // The range as a whole changes weeder.toml alone, because the file the
    // first commit adds the second takes away. The commits are what is
    // judged, and the first one touched something else.
    let repo = branch_over_an_old_law();
    repo.write("src/widened.ts", WIDENED);
    repo.commit("a parser of its own");
    repo.remove("src/widened.ts");
    repo.write("weeder.toml", MIGRATED);
    repo.commit(SIGNED);

    let run = repo.weeder(&["check", "--base", "main", "--strict", "--format", "sarif"]);

    assert_eq!(run.code, 3, "{}", run.output());
    assert!(run.stderr.contains(OF_ITS_OWN), "{}", run.stderr);
}

#[test]
fn a_push_that_only_migrates_the_law_passes_once_its_commit_is_signed() {
    let remote = Repo::bare();
    let repo = guarded_over_an_old_law();
    repo.git(&[
        "remote",
        "add",
        "origin",
        &remote.root().display().to_string(),
    ]);
    // The old law reached the remote the way it reached the branch: past a
    // weeder that would refuse it.
    repo.git(&["push", "--no-verify", "--set-upstream", "origin", "main"]);
    repo.write("weeder.toml", MIGRATED);
    repo.git(&["add", "weeder.toml"]);
    let landed = repo.try_git(&["commit", "-m", SIGNED]);
    assert_eq!(landed.code, 0, "the commit lands: {}", landed.output());

    let pushed = repo.try_git(&["push", "origin", "main"]);

    assert_eq!(
        pushed.code,
        0,
        "the signed migration leaves the machine: {}",
        pushed.output()
    );
    assert_eq!(
        remote.git(&["rev-parse", "main"]).trim(),
        repo.head(),
        "the remote carries the migration"
    );
    let said = pushed.output();
    assert!(
        said.contains(NOTICE),
        "the push says how it judged:\n{said}"
    );
}

#[test]
fn the_harness_hook_judges_a_staged_migration_rather_than_failing_to_run() {
    let repo = Repo::init();
    repo.write("weeder.toml", OLD_LAW);
    repo.commit("the law an older weeder read");
    repo.write("weeder.toml", MIGRATED);
    repo.git(&["add", "weeder.toml"]);
    let event = json!({
        "session_id": "a-session",
        "transcript_path": "/dev/null",
        "cwd": repo.root().display().to_string(),
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "git commit -m 'move the law'", "description": "commit"},
    })
    .to_string();

    let run = repo.weeder_reading(&["hook", "claude"], &event);

    assert_eq!(run.code, 2, "the hook denies the commit: {}", run.output());
    let said = run.output();
    assert!(said.contains("C1"), "the denial is C1's:\n{said}");
    assert!(
        !said.contains("is not valid"),
        "the denial is a judgement, not a run that never happened:\n{said}"
    );
}
