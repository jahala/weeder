//! `weed check` — the diff judged.
//!
//! With no flags weed judges the index plus the working tree against `HEAD`:
//! everything a worker changed, staged or not, which is what pleach's smoke gate
//! sees after it stages a node's files. `--staged` judges the index alone, the
//! view a pre-commit hook has. `--base <ref>` judges the tree against a ref, the
//! view CI has of a branch.
//!
//! A run that cannot reach a judgement — no repository, an unreadable ref, a
//! config weed cannot parse — leaves with exit 3 and says why on stderr. A gate
//! that could not run must never look like a gate that passed.

use std::path::{Path, PathBuf};

use crate::core::config::{parse_config, Config};
use crate::core::diff::{parse_diff, FileDiff};
use crate::core::finding::{Finding, Level};
use crate::core::sarif::{self, Context, EXIT_BLOCKED, EXIT_CLEAN, EXIT_COULD_NOT_RUN, RULES_DOC};
use crate::core::suppress::{
    apply_suppressions, parse_commit_suppressions, parse_inline_suppressions,
    InlineSuppressionError, Suppression,
};
use crate::core::{glob, rules};
use crate::faces::Answer;
use crate::seams::{fs, git};

/// How the findings are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Sarif,
    Table,
}

/// SARIF when stdout is not a terminal, a table when it is, and `--format`
/// overrides both. A machine reading a pipe gets the log; a person gets the table.
pub fn format_for(requested: Option<Format>, stdout_is_terminal: bool) -> Format {
    match requested {
        Some(format) => format,
        None if stdout_is_terminal => Format::Table,
        None => Format::Sarif,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Where weed was called from; the repository is found from here.
    pub cwd: PathBuf,
    /// Judge the tree against this ref instead of `HEAD`.
    pub base: Option<String>,
    /// Judge the index alone.
    pub staged: bool,
    /// Restrict the judged paths to these globs; empty leaves `weed.toml` in charge.
    pub scope: Vec<String>,
    /// Report suppressed findings at their own level, and refuse to guess.
    pub strict: bool,
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
    let config = read_config(request, &root)?;
    let diff = read_diff(request, &root)?;
    let files = parse_diff(&diff).map_err(|error| {
        format!("weed could not read the diff git produced: {error}. report it with the change that caused it.")
    })?;
    let judged: Vec<FileDiff> = in_scope(files, request, &config);

    let findings = rules::evaluate(&judged, &config);
    let (inline, malformed) = parse_inline_suppressions(&judged);
    if request.strict {
        if let Some(unreadable) = malformed.first() {
            return Err(refusal(unreadable));
        }
    }

    let findings = if request.strict {
        findings
    } else {
        let mut suppressions = inline;
        suppressions.extend(trailers(request, &root)?);
        apply_suppressions(findings, &suppressions)
    };

    let code = if findings.iter().any(|finding| finding.level == Level::Block) {
        EXIT_BLOCKED
    } else {
        EXIT_CLEAN
    };

    Ok(Answer {
        code,
        stdout: write(&findings, request, &root),
        stderr: malformed.iter().map(complaint).collect(),
    })
}

fn read_config(request: &Request, root: &Path) -> Result<Config, String> {
    let path = match &request.config {
        Some(path) => path.clone(),
        None => root.join("weed.toml"),
    };
    let text = match &request.config {
        // A config the caller pointed at and weed cannot read is a run that
        // never happened; a repository with no weed.toml simply takes the defaults.
        Some(_) => Some(
            fs::read(&path)
                .map_err(|error| format!("{error} --config must name a file weed can read."))?,
        ),
        None => fs::read_if_present(&path).map_err(|error| error.to_string())?,
    };
    parse_config(text.as_deref()).map_err(|error| {
        format!(
            "{} is not valid: {error}. fix that key, or drop it for the default.",
            path.display()
        )
    })
}

fn read_diff(request: &Request, root: &Path) -> Result<String, String> {
    let diff = match (&request.base, request.staged) {
        (Some(base), false) => git::diff_ref(root, base),
        (Some(_), true) => {
            return Err(
                "--base judges the tree against a ref and --staged judges the index, so weed cannot do both. pass one."
                    .to_string(),
            )
        }
        (None, true) => git::diff_index(root),
        (None, false) => git::diff_head(root),
    };
    diff.map_err(|error| error.to_string())
}

/// The files the run is allowed to judge. `--scope` wins where it is given;
/// otherwise `weed.toml`'s `[scope] allow` decides, and its default is every path.
fn in_scope(files: Vec<FileDiff>, request: &Request, config: &Config) -> Vec<FileDiff> {
    let globs = if request.scope.is_empty() {
        &config.scope_globs
    } else {
        &request.scope
    };
    files
        .into_iter()
        .filter(|file| {
            [file.new_path.as_deref(), file.old_path.as_deref()]
                .into_iter()
                .flatten()
                .any(|path| glob::matches_any(globs, path))
        })
        .collect()
}

/// The `Weed-allow:` trailers travelling with the change: the message of the
/// commit being prepared, and every commit a judged range carries.
fn trailers(request: &Request, root: &Path) -> Result<Vec<Suppression>, String> {
    let mut messages = Vec::new();
    if let Some(pending) = git::pending_commit_message(root).map_err(|error| error.to_string())? {
        messages.push(pending);
    }
    if let Some(base) = &request.base {
        messages.extend(git::commit_messages(root, base).map_err(|error| error.to_string())?);
    }
    Ok(messages
        .iter()
        .flat_map(|message| parse_commit_suppressions(message))
        .collect())
}

fn write(findings: &[Finding], request: &Request, root: &Path) -> String {
    match request.format {
        Format::Table => sarif::render_table(findings),
        Format::Sarif => {
            let mut context = Context::new(&request.version);
            if fs::exists(&root.join(RULES_DOC)) {
                context = context.with_docs_base(file_uri(root));
            }
            format!("{}\n", sarif::to_json(&sarif::render(findings, &context)))
        }
    }
}

fn could_not_run(request: &Request, reason: &str) -> Answer {
    let context = Context::new(&request.version).could_not_run(reason);
    Answer {
        code: EXIT_COULD_NOT_RUN,
        // A table has nothing to say about a run that never judged anything; the
        // log is the receipt a machine reads, and stderr is what a person reads.
        stdout: match request.format {
            Format::Sarif => format!("{}\n", sarif::to_json(&sarif::render(&[], &context))),
            Format::Table => String::new(),
        },
        stderr: vec![reason.to_string()],
    }
}

/// A `weed-allow` with no reason gives weed nothing to allow it against, so
/// under `--strict` the file goes unjudged rather than judged on a guess.
fn refusal(unreadable: &InlineSuppressionError) -> String {
    format!(
        "{} under --strict weed will not judge a file whose suppression it cannot read.",
        complaint(unreadable)
    )
}

fn complaint(error: &InlineSuppressionError) -> String {
    format!(
        "{}:{} carries a weed-allow weed cannot read: {}. write it as `weed-allow {}: the reason`.",
        error.path,
        error.line,
        error.message,
        if error.rule.is_empty() {
            "RULE"
        } else {
            &error.rule
        }
    )
}

/// The checkout as a `file:` URI, so `helpUri` resolves against this working
/// tree. A path is written as it is; SARIF wants an absolute URI, and an
/// absolute path is what makes one.
fn file_uri(root: &Path) -> String {
    format!("file://{}", root.display())
}
