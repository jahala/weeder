//! `weed bite`, the test held to its own failure.
//!
//! Every other face reads what a repository already holds. This one builds two
//! states it does not hold and runs the caller's test command over each: the
//! test commit alone on the base, where the command has to fail, and then the
//! implementation on top of that, where it has to pass. A command that passes
//! on the first state is a test that was green before the change existed, and
//! that is B1.
//!
//! The states are built in a worktree of weed's own, somewhere under the
//! machine's temporary directory, never in the caller's checkout: a judge that
//! moves a person's HEAD to ask a question is a judge nobody runs. The worktree
//! is taken away on the way out of every path, verdict or refusal, and the one
//! process weed starts is given a deadline it cannot outlive.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::core::bite::{Trial, Verdict};
use crate::core::catalogue::{self, Face};
use crate::core::change::Change;
use crate::core::diff::parse_diff;
use crate::core::finding::{Finding, Level};
use crate::core::rules;
use crate::core::sarif::{self, Context, EXIT_BLOCKED, EXIT_CLEAN, EXIT_COULD_NOT_RUN, RULES_DOC};
use crate::faces::{gather, read_config, Answer, Format, Source};
use crate::seams::{exec, fs, git};

/// The program the test command is handed to. A caller writes a command line,
/// with its own quoting and its own `&&`, and the thing that reads a command
/// line the way its author meant it is a shell. weed still hands it over as a
/// program and an argument array, so the line is data to everything but the
/// shell that was asked to read it.
const SHELL: &str = "sh";

/// How long weed waits for the test command where the caller names no deadline.
/// A suite is slower than everything else weed does by orders of magnitude, and
/// a suite that has not answered in five minutes is one nobody is waiting for.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

/// What the commit weed applies is called in the worktree it applies it in.
/// Nobody reads these commits; they exist so the next state can be built on top
/// of the one before it.
const TEST_PHASE: &str = "the test phase, as weed bite applied it";
const IMPLEMENTATION: &str = "the implementation, as weed bite applied it";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Where weed was called from; the repository is found from here.
    pub cwd: PathBuf,
    /// The command that runs the tests, as a command line.
    pub test: String,
    /// The state the test commit is applied to. Without it, two commits below
    /// the implementation.
    pub base: Option<String>,
    /// The commit carrying the tests. Without it, the commit below the
    /// implementation.
    pub test_commit: Option<String>,
    /// The commit carrying the change the tests cover. Without it, `HEAD`.
    pub impl_commit: Option<String>,
    /// How long the test command may take, in seconds.
    pub timeout: Option<u64>,
    pub format: Format,
    /// Read `weed.toml` from here instead of the repository root.
    pub config: Option<PathBuf>,
    pub version: String,
}

pub fn run(request: &Request) -> Answer {
    match judge(request) {
        Ok(answer) => answer,
        Err(reason) => could_not_run(request, &reason),
    }
}

fn judge(request: &Request) -> Result<Answer, String> {
    let root = git::repository_root(&request.cwd).map_err(|error| error.to_string())?;
    let config = read_config(&root, request.config.as_deref())?;
    let pair = resolve(&root, request)?;
    let tested = changes(&root, &pair)?;

    let at = worktree_path();
    git::add_worktree(&root, &at, &pair.base).map_err(|error| error.to_string())?;
    // Nothing between here and the sweep may leave with `?`: the worktree is
    // weed's to take away, and a run that walked out around this line would
    // leave a checkout of somebody's repository behind on their machine.
    let ran = trial(&at, &pair, request);
    let swept = git::remove_worktree(&root, &at);
    let alone = ran?;

    let findings = rules::bite::evaluate(
        &Trial {
            tested: &tested,
            alone,
        },
        &config,
    );
    let code = if findings.iter().any(|finding| finding.level == Level::Block) {
        EXIT_BLOCKED
    } else {
        EXIT_CLEAN
    };
    Ok(Answer {
        code,
        stdout: write(&findings, request, &root),
        stderr: swept
            .err()
            .into_iter()
            .map(|error| error.to_string())
            .collect(),
    })
}

/// The three commits a run judges, resolved to the shas the caller meant.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pair {
    /// The state the test commit is applied to.
    base: String,
    /// The commit carrying the tests.
    test: String,
    /// The commit carrying the change those tests cover.
    implementation: String,
}

/// Which commits this run is about. With nothing named, the two most recent
/// commits are the pair, the earlier of them being the test phase, and the
/// commit below them is the base. A ref this repository does not have is a run
/// that never happened rather than a guess at what was meant.
fn resolve(root: &Path, request: &Request) -> Result<Pair, String> {
    let implementation = request
        .impl_commit
        .clone()
        .unwrap_or_else(|| "HEAD".to_string());
    let test = request
        .test_commit
        .clone()
        .unwrap_or_else(|| format!("{implementation}~1"));
    let base = match (&request.base, &request.test_commit) {
        (Some(base), _) => base.clone(),
        (None, Some(named)) => format!("{named}~1"),
        (None, None) => format!("{implementation}~2"),
    };

    let pair = Pair {
        base: commit(root, &base)?,
        test: commit(root, &test)?,
        implementation: commit(root, &implementation)?,
    };
    if pair.base == pair.test || pair.test == pair.implementation {
        return Err(format!(
            "{base}, {test} and {implementation} do not name three different commits, so there is no test phase to hold to its own failure. name the pair with --test-commit and --impl-commit."
        ));
    }
    Ok(pair)
}

fn commit(root: &Path, reference: &str) -> Result<String, String> {
    git::resolve_ref(root, reference).map_err(|error| error.to_string())
}

/// What the test commit changed against the base, both sides of each file read.
/// This is what names the tests in a finding: the cases the commit added.
fn changes(root: &Path, pair: &Pair) -> Result<Vec<Change>, String> {
    let diff = git::diff_range(root, &pair.base, &pair.test).map_err(|error| error.to_string())?;
    let judged = parse_diff(&diff).map_err(|error| {
        format!("weed could not read the diff git produced: {error}. report it with the change that caused it.")
    })?;
    if judged.is_empty() {
        return Err(format!(
            "the commit weed was given as the test phase, {}, changes no file against {}. bite judges a test commit, so name one with --test-commit.",
            short(&pair.test),
            short(&pair.base)
        ));
    }
    gather(
        root,
        &pair.base,
        &Source::Reference(pair.test.clone()),
        judged,
    )
}

/// The two runs, in the worktree weed built for them. The first is the one that
/// decides: a command that passes with the tests alone has answered the whole
/// question, and the second run would only be asking whether an implementation
/// nobody needed also passes.
fn trial(at: &Path, pair: &Pair, request: &Request) -> Result<Verdict, String> {
    apply(at, &pair.test, TEST_PHASE, "the test phase")?;
    let alone = Verdict::of(command(at, request)?);
    if alone == Verdict::Passed {
        return Ok(alone);
    }

    apply(
        at,
        &pair.implementation,
        IMPLEMENTATION,
        "the implementation",
    )?;
    if Verdict::of(command(at, request)?) == Verdict::Failed {
        return Err(format!(
            "`{}` failed with {} applied on top of the test commit as well as without it, so bite has nothing to conclude about the tests. make the suite pass with the change, then run bite again.",
            request.test,
            short(&pair.implementation)
        ));
    }
    Ok(alone)
}

/// One commit's change, put into the worktree. A commit that will not apply
/// there is the state bite exists to build, so weed says which one and stops
/// rather than judging a state it did not build.
fn apply(at: &Path, commit: &str, message: &str, phase: &str) -> Result<(), String> {
    match git::apply_commit(at, commit, message).map_err(|error| error.to_string())? {
        git::Applied::Clean => Ok(()),
        git::Applied::Refused { message } => Err(format!(
            "weed could not apply {} as {phase}: {message}. bite needs a test commit that applies to the base on its own, which is what a conductor that commits its phases separately leaves behind.",
            short(commit)
        )),
    }
}

/// The test command, run once, in the state the worktree is holding.
fn command(at: &Path, request: &Request) -> Result<i32, String> {
    let timeout = request.timeout.map_or(DEFAULT_TIMEOUT, Duration::from_secs);
    match exec::run(at, SHELL, &["-c", &request.test], timeout) {
        Ok(output) => Ok(output.code),
        Err(exec::ExecError::TimedOut { seconds, .. }) => Err(format!(
            "`{}` was still running after {seconds}s and weed stopped waiting. give --timeout the seconds the suite needs, or name a command that answers in them.",
            request.test
        )),
        Err(error) => Err(format!(
            "{error} bite hands the test command to {SHELL}, which is what reads a command line."
        )),
    }
}

/// Where bite builds the states it runs. The name carries the process and the
/// moment it started, so two runs on one machine never meet in one directory,
/// and it sits under the machine's temporary directory rather than anywhere a
/// repository would notice.
fn worktree_path() -> PathBuf {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    std::env::temp_dir().join(format!("weed-bite-{}-{since}", std::process::id()))
}

/// A commit as a person reads one.
fn short(commit: &str) -> String {
    commit.chars().take(9).collect()
}

fn write(findings: &[Finding], request: &Request, root: &Path) -> String {
    match request.format {
        Format::Table => sarif::render_table(findings),
        Format::Sarif => {
            let mut context = Context::new(&request.version).with_catalogue(bite_catalogue());
            if fs::exists(&root.join(RULES_DOC)) {
                context = context.with_docs_base(format!("file://{}", root.display()));
            }
            format!("{}\n", sarif::to_json(&sarif::render(findings, &context)))
        }
    }
}

/// The rules a bite log declares: the ones this face can report.
fn bite_catalogue() -> Vec<catalogue::Rule> {
    catalogue::rules()
        .iter()
        .filter(|rule| rule.face == Face::Bite)
        .copied()
        .collect()
}

fn could_not_run(request: &Request, reason: &str) -> Answer {
    let context = Context::new(&request.version)
        .with_catalogue(bite_catalogue())
        .could_not_run(reason);
    Answer {
        code: EXIT_COULD_NOT_RUN,
        stdout: match request.format {
            Format::Sarif => format!("{}\n", sarif::to_json(&sarif::render(&[], &context))),
            Format::Table => String::new(),
        },
        stderr: vec![reason.to_string()],
    }
}
