//! C4, a gate was loosened in its own settings.
//!
//! An agent that cannot make a check pass can make the check unable to fail
//! instead, and from then on every check behind it is dark while the run stays
//! green. C3 says a workflow changed and T8 says a runner collects fewer files;
//! this names the loosening itself.
//!
//! Two shapes have no honest reading and block. A step a workflow already ran a
//! check in is told to let the check fail, by a setting on the step or its job,
//! `continue-on-error: true`, `if: false`, `allow_failure: true`, or by what is
//! written after the check's own command, `|| true`, `|| :`, `; exit 0`. And a
//! script a manifest or a makefile names as the check, `test`, `lint`,
//! `typecheck`, `check`, is turned into one that cannot fail: a command that
//! only prints or succeeds, the old command answered with `|| true`, or a
//! runner told to pass when it finds no tests.
//!
//! Two more shapes warn, because each has a reading a person can defend: a
//! gate's own setting moved the loose way, a coverage threshold lowered, a
//! strictness switch turned off, a code added to what a linter ignores, a lint
//! lowered to `allow`; and a check step taken out of a workflow that runs on a
//! push or a pull request, where no workflow the tree holds still runs it.
//!
//! A step is a check by the command it runs, read the way a shell reads it and
//! named by the words a check is called by, never by the action it uses: a
//! step that only `uses:` something is not judged. A setting is the gate's by
//! the words its key and the table it sits in are spelled with, so a dependency
//! that happens to be called `ignore` is nobody's lint.

use std::collections::BTreeSet;

use crate::core::change::{Change, Side};
use crate::core::classify::{classify_file, FileKind};
use crate::core::finding::{Detection, Level, Message, Region, Stamp};
use crate::core::rules::check::collect::{self, Entry, Stated};
use crate::core::rules::check::Judgement;
use crate::core::shell::{self, Joint, Linked};
use crate::core::syntax::Mask;

/// A workflow the tree holds that the change left alone, read only where a
/// change took a check step out of another, so a check that still runs
/// somewhere is not reported as gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workflow {
    pub path: String,
    pub syntax: Mask,
}

/// The words a check is called by. A command is a check where its program, or
/// the word it is told to run, is spelled with one of them: `pytest`, `cargo
/// test`, `npm run lint`, `golangci-lint`, `go vet`, `ruff check`.
const CHECKS: &[&str] = &["test", "lint", "check", "vet", "spec"];

/// The words a runner is told to run something else with, so the word after
/// them is what it runs: `npm run lint`, `poetry run pytest`.
const RUNS: &[&str] = &["run", "exec", "x"];

/// The shell's own words whose arguments are data rather than a check: the
/// conditional `test` and `[`, a word printed, a directory changed into.
const DATA: &[&str] = &[
    "test", "[", "[[", "echo", "printf", "cd", "true", ":", "exit", "export", "set",
];

/// The commands that cannot fail: what a check is answered with to stop it
/// failing, and what a script is left holding once it no longer runs one.
const SUCCEEDING: &[&str] = &["true", ":"];

/// The commands that only print.
const PRINTING: &[&str] = &["echo", "printf"];

/// The scripts a manifest or a makefile names as the check, without the `pre`
/// and `post` a hook is spelled with or the `:suffix` a variant carries.
const SCRIPTS: &[&str] = &["test", "lint", "typecheck", "check"];

/// The flag that tells a runner to pass when it finds nothing to run.
const PASS_WITH_NOTHING: &str = "--passWithNoTests";

/// The words a key, the table it sits in or the file it is written in is
/// spelled with when the setting belongs to a gate rather than to anything
/// else the file configures.
const GATES: &[&str] = &["lint", "check", "rule", "warn", "error"];

/// The words a key is spelled with when it lists what a gate looks away from.
const LOOKING_AWAY: &[&str] = &["ignore", "disable", "exclude"];

/// The levels a lint refuses or reports at, and the ones it is silent at.
const REPORTING: &[&str] = &["forbid", "deny", "warn", "warning", "error"];
const SILENT: &[&str] = &["allow", "off"];

/// The words a switch is spelled with when turning it on makes a checker
/// refuse more.
const STRICT: &[&str] = &["strict", "disallow"];

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let mut findings = Vec::new();
    let mut dropped: Vec<(String, String)> = Vec::new();
    for change in judged.changes {
        let Some(path) = change.path() else {
            continue;
        };
        if let Some(forge) = forge(path) {
            if let Some(new_path) = change.diff.new_path.as_deref() {
                findings.extend(steps_unable_to_fail(new_path, forge, change));
            }
            for name in checks_taken_out(forge, change) {
                dropped.push((path.to_string(), name));
            }
            continue;
        }
        if change.is_addition() || change.is_deletion() {
            continue;
        }
        if let Some(new_path) = change.diff.new_path.as_deref() {
            findings.extend(scripts_made_no_ops(new_path, change));
            findings.extend(settings_loosened(new_path, change));
        }
    }
    findings.extend(still_run_nowhere(&dropped, judged));
    findings
}

/// Whether a change takes anything out of a workflow, so the face reads the
/// workflows it left alone to see whether a check it took out still runs.
#[must_use]
pub fn reads_workflows(changes: &[Change]) -> bool {
    changes.iter().any(|change| {
        change.path().is_some_and(|path| forge(path).is_some()) && change.removed().next().is_some()
    })
}

/// Whether a path is a file a forge runs checks from: a workflow, or the
/// pipeline file a repository keeps at its root.
#[must_use]
pub fn is_workflow(path: &str) -> bool {
    forge(path).is_some()
}

// ---------------------------------------------------------------------------
// A workflow's steps
// ---------------------------------------------------------------------------

/// How a forge writes its steps down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Forge {
    /// Jobs under `jobs`, each holding a list of `steps`, a step running the
    /// command its `run` names.
    Steps,
    /// Jobs at the top of the file, each running the lines its `script` lists.
    Scripts,
}

fn forge(path: &str) -> Option<Forge> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let yaml = name.ends_with(".yml") || name.ends_with(".yaml");
    if yaml && classify_file(path, "").kind == FileKind::Workflow {
        return Some(Forge::Steps);
    }
    (path == ".gitlab-ci.yml" || path == ".gitlab-ci.yaml").then_some(Forge::Scripts)
}

/// One step of a workflow: the job it runs in and its place there, the lines
/// of shell it runs, and what it says about itself, its name and whether it
/// may fail among them.
#[derive(Debug, Clone, Default)]
struct Step {
    job: String,
    place: String,
    script: Vec<Entry>,
    settings: Vec<Stated>,
}

impl Step {
    /// What the step is called, where it says: its `name` or its `id`, or for
    /// a job that is one step, the job.
    fn label(&self) -> Option<&str> {
        self.settings
            .iter()
            .find(|setting| setting.key == "name" || setting.key == "id")
            .and_then(|setting| setting.entries.first())
            .map(|entry| entry.text.as_str())
    }

    fn written(&self) -> Vec<&str> {
        self.script.iter().map(|entry| entry.text.trim()).collect()
    }

    fn checks(&self) -> Vec<Check> {
        checks(&self.script)
    }

    fn names(&self) -> BTreeSet<String> {
        self.checks().into_iter().map(|check| check.name).collect()
    }
}

/// A workflow read as its jobs and their steps.
#[derive(Debug, Clone, Default)]
struct Run {
    steps: Vec<Step>,
    /// What each job says about failing, as one setting per key.
    jobs: Vec<(String, Vec<Stated>)>,
    /// Whether the workflow runs on a push or a pull request, which is what
    /// makes a check in it a gate rather than a run somebody asks for.
    gates: bool,
}

fn read(forge: Forge, path: &str, mask: &Mask) -> Run {
    let file = collect::File {
        path,
        kind: FileKind::Other,
        lang: crate::core::classify::Lang::Other,
        mask,
    };
    let stated = collect::stated(&file);
    match forge {
        Forge::Steps => read_steps(&stated),
        Forge::Scripts => read_scripts(&stated),
    }
}

fn read_steps(stated: &[Stated]) -> Run {
    let mut run = Run::default();
    for setting in stated {
        let scope: Vec<&str> = setting.scope.iter().map(String::as_str).collect();
        match scope.as_slice() {
            [] if setting.key == "on" => {
                run.gates |= setting.entries.iter().any(|entry| triggers(&entry.text));
            }
            ["on"] => run.gates |= triggers(&setting.key),
            ["jobs", job] => job_setting(&mut run.jobs, job, setting),
            ["jobs", job, "steps", place] => {
                let at = match run
                    .steps
                    .iter()
                    .position(|step| step.job == *job && step.place == *place)
                {
                    Some(at) => at,
                    None => {
                        run.steps.push(Step {
                            job: (*job).to_string(),
                            place: (*place).to_string(),
                            ..Step::default()
                        });
                        run.steps.len() - 1
                    }
                };
                let step = &mut run.steps[at];
                if setting.key == "run" {
                    step.script.extend(setting.entries.iter().cloned());
                } else {
                    step.settings.push(setting.clone());
                }
            }
            _ => {}
        }
    }
    run
}

/// Whether an event a workflow is started by is a push or a pull request.
fn triggers(event: &str) -> bool {
    matches!(event, "push" | "pull_request" | "pull_request_target")
}

fn job_setting(jobs: &mut Vec<(String, Vec<Stated>)>, job: &str, setting: &Stated) {
    match jobs.iter_mut().find(|(name, _)| name == job) {
        Some((_, held)) => held.push(setting.clone()),
        None => jobs.push((job.to_string(), vec![setting.clone()])),
    }
}

/// A pipeline file runs every job on a push, so each of its jobs is a gate. A
/// job is one step here: the lines its `script` lists, and its own settings.
fn read_scripts(stated: &[Stated]) -> Run {
    let mut run = Run {
        gates: true,
        ..Run::default()
    };
    for setting in stated {
        let [job] = setting.scope.as_slice() else {
            continue;
        };
        if setting.key == "script" {
            run.steps.push(Step {
                job: job.clone(),
                place: String::new(),
                script: setting.entries.clone(),
                settings: Vec::new(),
            });
        } else {
            job_setting(&mut run.jobs, job, setting);
        }
    }
    for step in &mut run.steps {
        if let Some((_, held)) = run.jobs.iter().find(|(name, _)| *name == step.job) {
            step.settings = held.clone();
        }
    }
    run.jobs.clear();
    run
}

/// What a step's or a job's own settings say to let its checks fail, and the
/// line that says it.
fn excused_by(settings: &[Stated]) -> Option<(&'static str, u32)> {
    settings.iter().find_map(|setting| {
        let spelled = match setting.key.as_str() {
            "continue-on-error" if says(setting, true) => "`continue-on-error: true`",
            "allow_failure" if says(setting, true) => "`allow_failure: true`",
            "if" if says(setting, false) => "`if: false`",
            _ => return None,
        };
        Some((spelled, setting.line))
    })
}

/// Whether a setting's value is this truth, written bare, quoted, or inside
/// the expression brackets a workflow evaluates.
fn says(setting: &Stated, wanted: bool) -> bool {
    if setting.truth == Some(wanted) {
        return true;
    }
    let [entry] = setting.entries.as_slice() else {
        return false;
    };
    let text = entry.text.trim();
    let text = text
        .strip_prefix("${{")
        .and_then(|inner| inner.strip_suffix("}}"))
        .unwrap_or(text)
        .trim();
    text == if wanted { "true" } else { "false" }
}

fn side_run(forge: Forge, path: &str, side: &Side) -> Option<Run> {
    side.content.as_ref()?;
    Some(read(forge, path, side.mask()))
}

/// The steps that ran a check before the change and can no longer fail after it.
fn steps_unable_to_fail(path: &str, forge: Forge, change: &Change) -> Vec<Detection> {
    let (Some(before), Some(after)) = (
        side_run(forge, path, &change.before),
        side_run(forge, path, &change.after),
    ) else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    let paired = counterparts(&before.steps, &after.steps);
    for (step, was) in after.steps.iter().zip(paired) {
        let names = step.names();
        let Some(was) = was.filter(|_| !names.is_empty()) else {
            continue;
        };
        let what = describe(&names, &step.job);
        if let Some((spelled, line)) = excused_by(&step.settings) {
            if excused_by(&was.settings).is_none() {
                findings.push(unable_to_fail(path, line, &what, spelled));
                continue;
            }
        }
        let before_checks = was.checks();
        for check in step.checks() {
            let Some(spelled) = check.excused else {
                continue;
            };
            let was_held = before_checks
                .iter()
                .any(|held| held.name == check.name && held.excused.is_none());
            if was_held {
                let what = describe(&BTreeSet::from([check.name.clone()]), &step.job);
                findings.push(unable_to_fail(path, check.line, &what, &spelled));
            }
        }
    }
    for (job, settings) in &after.jobs {
        let Some((spelled, line)) = excused_by(settings) else {
            continue;
        };
        let held_before = before
            .jobs
            .iter()
            .find(|(name, _)| name == job)
            .is_some_and(|(_, settings)| excused_by(settings).is_some());
        if held_before {
            continue;
        }
        // The job's checks that ran before the change, still in the job after it.
        let names: BTreeSet<String> = after
            .steps
            .iter()
            .filter(|step| step.job == *job)
            .flat_map(Step::names)
            .filter(|name| {
                before
                    .steps
                    .iter()
                    .filter(|step| step.job == *job)
                    .any(|step| step.names().contains(name))
            })
            .collect();
        if !names.is_empty() {
            let listed: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
            let what = format!("the `{job}` job, which runs {},", listed.join(" and "));
            findings.push(unable_to_fail(path, line, &what, spelled));
        }
    }
    findings
}

/// The step before the change that each step after it is, or nothing for a
/// step the change added. Each step before is one step after at most, in the
/// same job: a job that is a single step is itself; a step that says what it
/// is called is the step called that; and a step that does not is the one
/// that ran the same lines, or failing that the same checks.
fn counterparts<'a>(before: &'a [Step], after: &[Step]) -> Vec<Option<&'a Step>> {
    let mut taken = vec![false; before.len()];
    let mut paired: Vec<Option<&'a Step>> = vec![None; after.len()];
    type Same = fn(&Step, &Step) -> bool;
    let passes: [Same; 4] = [
        |was, now| was.place.is_empty() && now.place.is_empty(),
        |was, now| now.label().is_some() && was.label() == now.label(),
        |was, now| now.label().is_none() && was.label().is_none() && was.written() == now.written(),
        |was, now| {
            now.label().is_none()
                && was.label().is_none()
                && !now.names().is_empty()
                && was.names() == now.names()
        },
    ];
    for same in passes {
        for (at, now) in after.iter().enumerate() {
            if paired[at].is_some() {
                continue;
            }
            let found = before
                .iter()
                .enumerate()
                .find(|(index, was)| !taken[*index] && was.job == now.job && same(was, now));
            if let Some((index, was)) = found {
                taken[index] = true;
                paired[at] = Some(was);
            }
        }
    }
    paired
}

fn describe(names: &BTreeSet<String>, job: &str) -> String {
    let listed: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    format!(
        "the step running {} in the `{job}` job",
        listed.join(" and ")
    )
}

fn unable_to_fail(path: &str, line: u32, what: &str, spelled: &str) -> Detection {
    Detection {
        stamp: Stamp::Rule,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!("{what} was made unable to fail by {spelled}."),
            why: "its result no longer decides whether the run passes, so the run stays green whatever the check finds and every check behind it goes dark.".to_string(),
            next: "take the loosening back out and fix what fails, or write why the check may fail in a `Weeder-allow: C4` trailer.".to_string(),
        },
        fix: None,
    }
}

/// The checks a workflow that gates ran before the change and does not run after it.
fn checks_taken_out(forge: Forge, change: &Change) -> Vec<String> {
    let Some(path) = change.diff.old_path.as_deref() else {
        return Vec::new();
    };
    let Some(before) = side_run(forge, path, &change.before) else {
        return Vec::new();
    };
    if !before.gates {
        return Vec::new();
    }
    let after = change
        .diff
        .new_path
        .as_deref()
        .and_then(|new_path| side_run(forge, new_path, &change.after))
        .unwrap_or_default();
    let kept: BTreeSet<String> = after.steps.iter().flat_map(Step::names).collect();
    let ran: BTreeSet<String> = before.steps.iter().flat_map(Step::names).collect();
    ran.difference(&kept).cloned().collect()
}

/// The checks taken out of a workflow that no workflow of the tree runs any
/// more: neither the after side of a workflow the change touched, nor one it
/// left alone.
fn still_run_nowhere(dropped: &[(String, String)], judged: &Judgement) -> Vec<Detection> {
    if dropped.is_empty() {
        return Vec::new();
    }
    let mut running: BTreeSet<String> = BTreeSet::new();
    for change in judged.changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if let Some(run) = forge(path).and_then(|forge| side_run(forge, path, &change.after)) {
            running.extend(run.steps.iter().flat_map(Step::names));
        }
    }
    for workflow in judged.workflows {
        if let Some(forge) = forge(&workflow.path) {
            let run = read(forge, &workflow.path, &workflow.syntax);
            running.extend(run.steps.iter().flat_map(Step::names));
        }
    }
    dropped
        .iter()
        .filter(|(_, name)| !running.contains(name))
        .map(|(path, name)| Detection {
            stamp: Stamp::Override(Level::Warn),
            path: path.clone(),
            region: None,
            message: Message {
                what: format!("the step running `{name}` was taken out of a workflow that runs on a push or a pull request, and no workflow runs it now."),
                why: "the check no longer runs before a change lands, so what it used to catch now lands unseen.".to_string(),
                next: "put the step back or move it to another workflow, or write why it is gone in a `Weeder-allow: C4` trailer.".to_string(),
            },
            fix: None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The commands a script runs
// ---------------------------------------------------------------------------

/// One check a script runs: the name it is run under, the line it is written
/// on, and what the script wrote to let it fail, where it wrote anything.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Check {
    name: String,
    line: u32,
    excused: Option<String>,
}

/// The lines of a script read as the command lines a shell runs: a line that
/// ends with a backslash or an operator carries on onto the next.
fn command_lines(script: &[Entry]) -> Vec<(String, u32)> {
    let mut lines: Vec<(String, u32)> = Vec::new();
    let mut carried = false;
    for entry in script {
        let text = entry.text.trim();
        match lines.last_mut() {
            Some((held, _)) if carried => {
                held.push(' ');
                held.push_str(text.trim_end_matches('\\'));
            }
            _ => lines.push((text.trim_end_matches('\\').to_string(), entry.line)),
        }
        carried = text.ends_with('\\')
            || text.ends_with("||")
            || text.ends_with("&&")
            || text.ends_with('|');
    }
    lines
}

fn checks(script: &[Entry]) -> Vec<Check> {
    let mut found = Vec::new();
    for (line, number) in command_lines(script) {
        let chain = shell::chain(&line);
        for (at, link) in chain.iter().enumerate() {
            let Some(name) = check_name(&link.words) else {
                continue;
            };
            found.push(Check {
                name,
                line: number,
                excused: excused_after(&chain, at),
            });
        }
    }
    found
}

/// What a command line writes after the command at `at` that makes it unable
/// to fail: a command that always succeeds after `||`, which answers a failure
/// anywhere in the list it closes, or `; exit 0` after it on the same line.
fn excused_after(chain: &[Linked], at: usize) -> Option<String> {
    let mut joined = true;
    for link in &chain[at + 1..] {
        let (words, _) = shell::simple(&link.words);
        match link.joint {
            Joint::Or if joined && succeeds(words) => {
                return Some(format!("`|| {}`", words.join(" ")));
            }
            Joint::Then if is_exit_zero(words) => return Some("`; exit 0`".to_string()),
            Joint::Then | Joint::Start | Joint::Apart => joined = false,
            Joint::And | Joint::Or | Joint::Pipe => {}
        }
    }
    None
}

/// Whether a command cannot fail.
fn succeeds(words: &[String]) -> bool {
    let program = shell::program(words);
    SUCCEEDING.contains(&program) || is_exit_zero(words)
}

fn is_exit_zero(words: &[String]) -> bool {
    shell::program(words) == "exit" && words.get(1).is_none_or(|code| code == "0")
}

/// The name a command is a check under, or nothing where it checks nothing:
/// its program, and the word it is told to run where it is told one, `npm
/// test`, `cargo test`, `go vet`, `pytest`, `python -m pytest`, `npm run
/// lint`. A path, an option or a value is not a name.
fn check_name(words: &[String]) -> Option<String> {
    let (words, _) = shell::simple(words);
    let program = shell::program(words);
    if program.is_empty() || DATA.contains(&program) {
        return None;
    }
    let mut named = vec![program.to_string()];
    let mut at = 1;
    while let Some(word) = words.get(at) {
        at += 1;
        // A module handed to an interpreter is the program it runs.
        if word == "-m" {
            continue;
        }
        if word.starts_with('-') || word.contains(['/', '.', '=', '@', '$']) {
            break;
        }
        named.push(word.clone());
        if !RUNS.contains(&word.as_str()) {
            break;
        }
    }
    let is_check =
        named.iter().any(|name| is_check_word(name)) || words.iter().any(|word| word == "--check");
    is_check.then(|| named.join(" "))
}

/// Whether a name is spelled with a word a check is called by: one of its
/// parts, split at a dash, an underscore or a colon, ends with one.
fn is_check_word(name: &str) -> bool {
    name.to_ascii_lowercase()
        .split(['-', '_', ':'])
        .any(|part| CHECKS.iter().any(|check| part.ends_with(check)))
}

/// What a command is, for telling the same command apart on two sides of a
/// change: the name it is a check under, or the program it runs.
fn identity(words: &[String]) -> String {
    check_name(words).unwrap_or_else(|| shell::program(shell::simple(words).0).to_string())
}

/// Whether a command does something a script can fail at: anything but a word
/// printed, a command that always succeeds, and an exit.
fn does_something(words: &[String]) -> bool {
    let (words, _) = shell::simple(words);
    let program = shell::program(words);
    !program.is_empty()
        && !SUCCEEDING.contains(&program)
        && !PRINTING.contains(&program)
        && program != "exit"
}

// ---------------------------------------------------------------------------
// A manifest's or a makefile's check scripts
// ---------------------------------------------------------------------------

/// One script a manifest or a makefile names: what it is called, the line it is
/// named on, the lines it runs, and the lines make was told to ignore the
/// failure of.
#[derive(Debug, Clone)]
struct Script {
    name: String,
    line: u32,
    lines: Vec<Entry>,
    ignoring: bool,
}

/// Whether a script's name is one a check runs under.
fn is_check_script(name: &str) -> bool {
    let base = name.split(':').next().unwrap_or(name);
    let base = ["pre", "post"]
        .iter()
        .find_map(|hook| {
            base.strip_prefix(hook)
                .filter(|rest| SCRIPTS.contains(rest))
        })
        .unwrap_or(base);
    SCRIPTS.contains(&base)
}

fn scripts_of(path: &str, side: &Side) -> Vec<Script> {
    if side.content.is_none() {
        return Vec::new();
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    if matches!(name, "Makefile" | "makefile" | "GNUmakefile") || name.ends_with(".mk") {
        return targets(side.mask());
    }
    if side.kind() != FileKind::Manifest {
        return Vec::new();
    }
    let file = collect::File {
        path,
        kind: side.kind(),
        lang: side.lang(),
        mask: side.mask(),
    };
    collect::stated(&file)
        .into_iter()
        .filter(|setting| {
            setting.scope.last().is_some_and(|table| {
                let table = table.rsplit('.').next().unwrap_or(table);
                table == "scripts" || table == "tasks"
            })
        })
        .filter(|setting| is_check_script(&setting.key))
        .filter_map(|setting| {
            let [entry] = setting.entries.as_slice() else {
                return None;
            };
            // A name bound to `module:function` is a program an installer
            // writes, not a command line.
            let text = entry.text.trim();
            if !text.contains(char::is_whitespace) && text.contains(':') && !text.contains('/') {
                return None;
            }
            Some(Script {
                name: setting.key.clone(),
                line: setting.line,
                lines: vec![entry.clone()],
                ignoring: false,
            })
        })
        .collect()
}

/// The targets a makefile names and the recipe lines under each, the prefix
/// make reads off a recipe line taken off and remembered: a `-` there tells
/// make to carry on whatever the line did.
fn targets(mask: &Mask) -> Vec<Script> {
    let mut found: Vec<Script> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    for line in 1..=mask.line_count() {
        let text = mask.outside_comments(line);
        if let Some(recipe) = text.strip_prefix('\t') {
            let bare = recipe.trim_start_matches(['@', '-', '+', ' ']);
            let ignoring = recipe[..recipe.len() - bare.len()].contains('-');
            for at in &current {
                found[*at].lines.push(Entry {
                    text: bare.to_string(),
                    line,
                });
                found[*at].ignoring |= ignoring;
            }
            continue;
        }
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        current.clear();
        let Some((names, rest)) = trimmed.split_once(':') else {
            continue;
        };
        if rest.starts_with('=') || names.contains(['=', '$']) {
            continue;
        }
        for name in names.split_whitespace() {
            if is_check_script(name) {
                found.push(Script {
                    name: name.to_string(),
                    line,
                    lines: Vec::new(),
                    ignoring: false,
                });
                current.push(found.len() - 1);
            }
        }
    }
    found
}

/// The check scripts that ran a command before the change and cannot fail after it.
fn scripts_made_no_ops(path: &str, change: &Change) -> Vec<Detection> {
    let Some(old_path) = change.diff.old_path.as_deref() else {
        return Vec::new();
    };
    let before = scripts_of(old_path, &change.before);
    if before.is_empty() {
        return Vec::new();
    }
    let mut findings = Vec::new();
    for script in scripts_of(path, &change.after) {
        let Some(was) = before.iter().find(|was| was.name == script.name) else {
            continue;
        };
        if let Some(how) = no_op(was, &script) {
            findings.push(Detection {
                stamp: Stamp::Rule,
                path: path.to_string(),
                region: Some(Region {
                    start_line: script.line,
                    end_line: script.line,
                }),
                message: Message {
                    what: format!("the `{}` script was made unable to fail: {how}.", script.name),
                    why: "everything that runs the script, a hook, a workflow or a person, now reads a pass whatever the code does.".to_string(),
                    next: "restore the command it ran, or write why the script may not fail in a `Weeder-allow: C4` trailer.".to_string(),
                },
                fix: None,
            });
        }
    }
    findings
}

/// How a script that ran a command became one that cannot fail, or nothing
/// where it still can.
fn no_op(was: &Script, now: &Script) -> Option<String> {
    let lines = |script: &Script| -> Vec<Vec<Linked>> {
        command_lines(&script.lines)
            .iter()
            .map(|(line, _)| shell::chain(line))
            .collect()
    };
    let (before, after) = (lines(was), lines(now));
    let ran = |chains: &[Vec<Linked>]| {
        chains
            .iter()
            .flatten()
            .any(|link| does_something(&link.words))
    };
    if !ran(&before) || was.ignoring {
        return None;
    }
    let written = |script: &Script| {
        script
            .lines
            .iter()
            .map(|entry| entry.text.trim().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    };
    let fails = after.iter().flatten().any(|link| {
        let (words, _) = shell::simple(&link.words);
        shell::program(words) == "exit" && !is_exit_zero(words)
    });
    if !ran(&after) && !fails {
        return Some(format!("`{}` cannot fail", written(now)));
    }
    if now.ignoring {
        return Some("make is told to ignore its failure with `-`".to_string());
    }
    // The commands the script ran before the change and let fail: an old
    // command answered with `|| true` is the shape, and a new command written
    // beside the old one as advice leaves the old one to fail as it did.
    let held: BTreeSet<String> = before
        .iter()
        .flat_map(|chain| {
            chain
                .iter()
                .enumerate()
                .filter(move |(at, link)| {
                    does_something(&link.words) && excused_after(chain, *at).is_none()
                })
                .map(|(_, link)| identity(&link.words))
        })
        .collect();
    let excused = after.iter().find_map(|chain| {
        chain.iter().enumerate().find_map(|(at, link)| {
            (does_something(&link.words) && held.contains(&identity(&link.words)))
                .then(|| excused_after(chain, at))
                .flatten()
        })
    });
    if let Some(spelled) = excused {
        return Some(format!("its command is answered with {spelled}"));
    }
    let passes_empty = |chains: &[Vec<Linked>]| {
        chains
            .iter()
            .flatten()
            .any(|link| link.words.iter().any(|word| word == PASS_WITH_NOTHING))
    };
    (passes_empty(&after) && !passes_empty(&before))
        .then(|| format!("`{PASS_WITH_NOTHING}` tells the runner to pass when it finds no tests"))
}

// ---------------------------------------------------------------------------
// A gate's own settings
// ---------------------------------------------------------------------------

/// The settings a change moved the way that lets a gate refuse less.
fn settings_loosened(path: &str, change: &Change) -> Vec<Detection> {
    let Some(old_path) = change.diff.old_path.as_deref() else {
        return Vec::new();
    };
    let read = |path: &str, side: &Side| -> Vec<Stated> {
        if side.content.is_none() {
            return Vec::new();
        }
        collect::stated(&collect::File {
            path,
            kind: side.kind(),
            lang: side.lang(),
            mask: side.mask(),
        })
    };
    let before = read(old_path, &change.before);
    if before.is_empty() {
        return Vec::new();
    }
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    let mut findings = Vec::new();
    for setting in read(path, &change.after) {
        let was = before
            .iter()
            .find(|was| was.key == setting.key && was.scope == setting.scope);
        if let Some(how) = loosened(&name, was, &setting) {
            findings.push(Detection {
                stamp: Stamp::Override(Level::Warn),
                path: path.to_string(),
                region: Some(Region {
                    start_line: setting.line,
                    end_line: setting.line,
                }),
                message: how,
                fix: None,
            });
        }
    }
    findings
}

/// A setting's name and every name it sits under, run together in lower case
/// and without the dashes and underscores the formats spell words apart with.
fn spelled(setting: &Stated) -> String {
    setting
        .scope
        .iter()
        .chain(std::iter::once(&setting.key))
        .map(|name| name.to_ascii_lowercase().replace(['-', '_'], ""))
        .collect::<Vec<_>>()
        .join(".")
}

/// The setting as a reader finds it in the file: the names it sits under and
/// its own, joined with dots, a list item left out.
fn dotted(setting: &Stated) -> String {
    setting
        .scope
        .iter()
        .chain(std::iter::once(&setting.key))
        .filter(|name| !name.is_empty() && !name.starts_with('['))
        .cloned()
        .collect::<Vec<_>>()
        .join(".")
}

fn loosened(file: &str, was: Option<&Stated>, now: &Stated) -> Option<Message> {
    let words = spelled(now);
    let key = now.key.to_ascii_lowercase().replace(['-', '_'], "");
    let gate = GATES
        .iter()
        .any(|word| words.contains(word) || file.contains(word));
    let shown = dotted(now);
    let next = "restore the setting, or write why the gate may refuse less in a `Weeder-allow: C4` trailer.".to_string();

    if LOOKING_AWAY.iter().any(|word| key.contains(word)) && gate {
        let held: BTreeSet<String> = was.map(|was| codes(&was.entries)).unwrap_or_default();
        let added: Vec<String> = codes(&now.entries)
            .into_iter()
            .filter(|code| !held.contains(code))
            .collect();
        if added.is_empty() {
            return None;
        }
        let listed: Vec<String> = added.iter().map(|code| format!("`{code}`")).collect();
        return Some(Message {
            what: format!("{} added to `{shown}`.", phrase(&listed)),
            why: "the linter still runs and no longer reports what those codes name, so the code they catch lands unread.".to_string(),
            next: "take the codes back out, or write why the linter may look away from them in a `Weeder-allow: C4` trailer.".to_string(),
        });
    }
    let was = was?;
    if let (Some(before), Some(after)) = (number(was), number(now)) {
        if after < before && is_threshold(file, &words) {
            return Some(Message {
                what: format!(
                    "`{shown}` was lowered from {} to {}.",
                    was.number.as_deref().unwrap_or_default(),
                    now.number.as_deref().unwrap_or_default()
                ),
                why: "the coverage gate still runs and now passes code it used to refuse."
                    .to_string(),
                next,
            });
        }
        return None;
    }
    if was.truth == Some(true)
        && now.truth == Some(false)
        && STRICT.iter().any(|word| key.contains(word))
    {
        return Some(Message {
            what: format!("`{shown}` was turned off."),
            why: "the checker still runs and now lets through what the switch made it refuse."
                .to_string(),
            next,
        });
    }
    let (before, after) = (first_text(was)?, first_text(now)?);
    if before == "strict" && after != "strict" {
        return Some(Message {
            what: format!("`{shown}` was moved from `strict` to `{after}`."),
            why: "the checker still runs and now lets through what the strict mode made it refuse."
                .to_string(),
            next,
        });
    }
    if gate && REPORTING.contains(&before.as_str()) && SILENT.contains(&after.as_str()) {
        return Some(Message {
            what: format!("`{shown}` was lowered from `{before}` to `{after}`."),
            why: "the lint still runs and no longer reports what it was set to refuse.".to_string(),
            next,
        });
    }
    None
}

/// The codes a list of entries names, an entry written as a line of them read
/// as each code it holds.
fn codes(entries: &[Entry]) -> BTreeSet<String> {
    entries
        .iter()
        .flat_map(|entry| {
            entry
                .text
                .split([',', ' ', '\t'])
                .map(str::trim)
                .filter(|code| !code.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn phrase(listed: &[String]) -> String {
    match listed {
        [one] => format!("{one} was"),
        [rest @ .., last] => format!("{} and {last} were", rest.join(", ")),
        [] => String::new(),
    }
}

fn number(setting: &Stated) -> Option<f64> {
    setting.number.as_deref()?.parse().ok()
}

fn first_text(setting: &Stated) -> Option<String> {
    setting
        .entries
        .first()
        .map(|entry| entry.text.trim().to_ascii_lowercase())
}

/// Whether a number is a coverage threshold: the setting or its file says
/// coverage and the setting says threshold or minimum, or the key says the
/// run fails under it.
fn is_threshold(file: &str, words: &str) -> bool {
    let covered = words.contains("cover") || file.contains("cover");
    let bound = words.contains("threshold") || words.contains("minimum") || words.contains(".min");
    words.contains("failunder") || (covered && bound)
}
