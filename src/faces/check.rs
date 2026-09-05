//! `weed check`, the diff judged.
//!
//! With no flags weed judges the index plus the working tree against `HEAD`:
//! everything a worker changed, staged or not, which is what pleach's smoke gate
//! sees after it stages a node's files. `--staged` judges the index alone, the
//! view a pre-commit hook has. `--base <ref>` judges the tree against a ref, the
//! view CI has of a branch.
//!
//! A run that cannot reach a judgement, no repository, an unreadable ref, a
//! config weed cannot parse, leaves with exit 3 and says why on stderr. A gate
//! that could not run must never look like a gate that passed.

use std::path::{Path, PathBuf};

use crate::core::change::{Change, Side};
use crate::core::classify::{classify_file, FileKind};
use crate::core::diff::{parse_diff, FileDiff};
use crate::core::finding::{Finding, Level};
use crate::core::read::TestShape;
use crate::core::rules;
use crate::core::sarif::{self, Context, EXIT_BLOCKED, EXIT_CLEAN, EXIT_COULD_NOT_RUN, RULES_DOC};
use crate::core::suppress::{
    apply_suppressions, parse_commit_suppressions, parse_inline_suppressions,
    InlineSuppressionError, Suppression,
};
use crate::faces::{read_config, Answer, Format};
use crate::seams::{fs, git, reader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Where weed was called from; the repository is found from here.
    pub cwd: PathBuf,
    /// Judge the tree against this ref instead of `HEAD`.
    pub base: Option<String>,
    /// Judge this ref against `base` rather than the working tree. It is the
    /// view a pre-push hook needs, which must judge the commits being pushed
    /// and not whatever the checkout happens to be holding.
    pub tip: Option<String>,
    /// Judge the index alone.
    pub staged: bool,
    /// The paths a change may touch. Every file is judged whatever the scope;
    /// X2 (rules-prod) reports a file outside it. Empty leaves `weed.toml` in charge.
    pub scope: Vec<String>,
    /// Report suppressed findings at their own level, and refuse to guess.
    pub strict: bool,
    pub format: Format,
    /// Read `weed.toml` from here instead of the repository root.
    pub config: Option<PathBuf>,
    /// The message of the commit being prepared, for its `Weed-allow:` trailers.
    /// A pre-commit gate has no commit to read, so a hook hands the message in.
    pub message_file: Option<PathBuf>,
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
    let range = range(request)?;
    let diff = read_diff(&root, &range)?;
    let judged: Vec<FileDiff> = parse_diff(&diff).map_err(|error| {
        format!("weed could not read the diff git produced: {error}. report it with the change that caused it.")
    })?;
    let (inline, malformed) = parse_inline_suppressions(&judged);

    let findings = rules::check::evaluate(&gather(&root, &range, judged)?, &config);
    if request.strict {
        if let Some(unreadable) = malformed.first() {
            return Err(refusal(unreadable));
        }
    }

    let mut suppressions = inline;
    suppressions.extend(trailers(request, &root)?);
    let findings = apply_suppressions(findings, &suppressions);
    // Under --strict a suppression is reported and not honoured: the finding
    // keeps the suppression it carries and gets its own level back, so a gate an
    // agent wrote an allowance for still stops it, and the reviewer sees both.
    let findings: Vec<Finding> = if request.strict {
        findings
            .into_iter()
            .map(|mut finding| {
                if let Some(level) = finding.suppressed.as_ref().and_then(|s| s.original_level) {
                    finding.level = level;
                }
                finding
            })
            .collect()
    } else {
        findings
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

/// The two states a run compares. The state a change starts from is always a
/// commit, git has nothing else to compare against, and what it ends in is
/// whichever of the three places the caller asked weed to judge.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Range {
    base: String,
    after: Source,
}

/// Where a version of a file is to be found.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    /// A commit, by whatever name the caller gave it.
    Reference(String),
    /// The index: what a commit would carry.
    Index,
    /// The working tree, as it sits on disk.
    Tree,
}

/// What this run compares against what. A run that names two states weed cannot
/// judge together stops here rather than guessing which one was meant.
fn range(request: &Request) -> Result<Range, String> {
    if request.staged && (request.base.is_some() || request.tip.is_some()) {
        return Err(
            "--base judges the tree against a ref and --staged judges the index, so weed cannot do both. pass one."
                .to_string(),
        );
    }
    if request.base.is_none() && request.tip.is_some() {
        return Err(
            "a tip with no base names no range to judge, and weed will not guess one.".to_string(),
        );
    }
    let base = request.base.clone().unwrap_or_else(|| "HEAD".to_string());
    let after = match (&request.tip, request.staged) {
        (Some(tip), _) => Source::Reference(tip.clone()),
        (None, true) => Source::Index,
        (None, false) => Source::Tree,
    };
    Ok(Range { base, after })
}

fn read_diff(root: &Path, range: &Range) -> Result<String, String> {
    let head = range.base == "HEAD";
    let diff = match &range.after {
        Source::Reference(tip) => git::diff_range(root, &range.base, tip),
        Source::Index if head => git::diff_index(root),
        Source::Tree if head => git::diff_head(root),
        // A base other than HEAD is a ref, and git compares the tree against a
        // ref the same way whether or not the index is part of the question.
        _ => git::diff_ref(root, &range.base),
    };
    diff.map_err(|error| error.to_string())
}

/// Each changed file with both of its sides read: the text, what the path is,
/// and what the reader makes of the inside of it. This is the one place weed
/// touches a file for the rules, so a detector stays pure and testable whole.
fn gather(root: &Path, range: &Range, judged: Vec<FileDiff>) -> Result<Vec<Change>, String> {
    judged
        .into_iter()
        .map(|diff| {
            let before = side(
                root,
                &Source::Reference(range.base.clone()),
                diff.old_path.as_deref(),
            )?;
            let after = side(root, &range.after, diff.new_path.as_deref())?;
            Ok(Change {
                diff,
                before,
                after,
            })
        })
        .collect()
}

/// One side of one file. A side with no file, and a file that is not text, are
/// both nothing to read: weed judges lines, and neither carries any.
fn side(root: &Path, source: &Source, path: Option<&str>) -> Result<Side, String> {
    let Some(path) = path else {
        return Ok(Side::default());
    };
    let content = match source {
        Source::Reference(reference) => git::file_at_ref(root, reference, path),
        Source::Index => git::file_in_index(root, path),
        Source::Tree => git::file_in_tree(root, path),
    }
    .map_err(|error| error.to_string())?;
    let Some(content) = content else {
        return Ok(Side::default());
    };

    let classification = classify_file(path, &content);
    let file = Path::new(path);
    let tests = if classification.kind == FileKind::Test || classification.has_inline_tests {
        reader::test_shape(file, &content)
    } else {
        TestShape::default()
    };
    // Every side is outlined, test file and production file alike: a rule that
    // asks what a rename did to a case, or which unit a double stands in for,
    // is asking about a declaration on whichever side of the suite it sits.
    let outline = reader::outline(file, &content);
    Ok(Side {
        content: Some(content),
        classification: Some(classification),
        tests,
        outline,
    })
}

/// The `Weed-allow:` trailers travelling with the change: the message handed in
/// with `--message-file`, and every commit a judged range carries. git's own
/// COMMIT_EDITMSG is never read: at pre-commit time it still holds the previous
/// commit's message, and a trailer must never outlive the change it was written for.
fn trailers(request: &Request, root: &Path) -> Result<Vec<Suppression>, String> {
    let mut messages = Vec::new();
    if let Some(path) = &request.message_file {
        messages.push(
            fs::read(path).map_err(|error| {
                format!("{error} --message-file must name a file weed can read.")
            })?,
        );
    }
    if let Some(base) = &request.base {
        let tip = request.tip.as_deref().unwrap_or("HEAD");
        messages.extend(git::commit_messages(root, base, tip).map_err(|error| error.to_string())?);
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
