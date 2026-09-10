//! `weeder check`, the diff judged.
//!
//! With no flags weeder judges the index plus the working tree against `HEAD`:
//! everything a worker changed, staged or not, which is what pleach's smoke gate
//! sees after it stages a node's files. `--staged` judges the index alone, the
//! view a pre-commit hook has. `--base <ref>` judges the tree against a ref, the
//! view CI has of a branch.
//!
//! A file git has never been told about is in none of those diffs, and most of
//! what an agent writes is one. So the default mode reads them too, as what they
//! are: added files, with nothing before them. `--untracked` says otherwise
//! either way, and the index and a range keep their own meanings, because an
//! index holds what it holds and a history has no working tree in it.
//!
//! A run that cannot reach a judgement, no repository, an unreadable ref, a
//! config weeder cannot parse, leaves with exit 3 and says why on stderr. A gate
//! that could not run must never look like a gate that passed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::core::change::{self, Change, Side};
use crate::core::classify::{classify_file, FileKind};
use crate::core::config::Config;
use crate::core::diff::{added_file, parse_diff, FileDiff};
use crate::core::finding::{Finding, Level};
use crate::core::glob;
use crate::core::hierarchy::{self, Hierarchy};
use crate::core::read::CallerSite;
use crate::core::rules;
use crate::core::rules::check::collect;
use crate::core::sarif::{self, Context, EXIT_BLOCKED, EXIT_CLEAN, EXIT_COULD_NOT_RUN, RULES_DOC};
use crate::core::specimen;
use crate::core::suppress::{
    apply_suppressions, parse_commit_suppressions, parse_inline_suppressions,
    InlineSuppressionError, Suppression, SuppressionSource, MARKER,
};
use crate::core::syntax::Mask;
use crate::faces::{blobs, gather, read_config, side, Answer, Format, Source, Untracked};
use crate::seams::{fs, git, reader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Where weeder was called from; the repository is found from here.
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
    /// X2 (rules-prod) reports a file outside it. Empty leaves `weeder.toml` in charge.
    pub scope: Vec<String>,
    /// Whether the files git has never been told about are part of the change.
    /// `None` leaves it to the mode: they are, wherever the working tree is
    /// being judged against `HEAD`, and are not anywhere else.
    pub untracked: Option<Untracked>,
    /// Report suppressed findings at their own level, and refuse to guess.
    pub strict: bool,
    /// Whether `--strict` still honours an allowance a person wrote in a commit
    /// message. It is for the caller that judges a change the message travels
    /// with: the commit-msg hook reads the message being written, the pre-push
    /// hook reads the messages of the commits being pushed, and in both the
    /// trailer is a person's act on this very change. An inline marker is the
    /// agent's own line and is never honoured here, whatever this says.
    pub honour_trailers: bool,
    pub format: Format,
    /// Read `weeder.toml` from here instead of the repository root.
    pub config: Option<PathBuf>,
    /// The message of the commit being prepared, for its `Weeder-allow:` trailers.
    /// A pre-commit gate has no commit to read, so a hook hands the message in.
    pub message_file: Option<PathBuf>,
    pub version: String,
}

/// What a run judged: the bytes a caller writes and the code it leaves with,
/// and the findings behind them. A face that only reports takes the answer; the
/// pre-commit hook takes the findings too, because it names the trailer that
/// would allow each rule that blocked, and a rule id read back out of a rendered
/// table would be weeder parsing its own prose.
pub struct Verdict {
    pub answer: Answer,
    pub findings: Vec<Finding>,
}

pub fn run(request: &Request) -> Answer {
    verdict(request).answer
}

pub fn verdict(request: &Request) -> Verdict {
    match judge(request) {
        Ok(verdict) => verdict,
        // A run that never judged anything has no findings to hand over, and
        // saying so with an empty list is what keeps a caller from reading a
        // failure as a clean tree.
        Err(reason) => Verdict {
            answer: could_not_run(request, &reason),
            findings: Vec::new(),
        },
    }
}

fn judge(request: &Request) -> Result<Verdict, String> {
    let root = git::repository_root(&request.cwd).map_err(|error| error.to_string())?;
    let config = read_config(&root, request.config.as_deref())?;
    let range = range(request)?;
    // Asked before anything is judged: a run that cannot do what it was asked
    // never happened, and refusing after the work is a refusal that cost time.
    let untracked = reads_untracked(request, &range)?;
    let diff = read_diff(&root, &range)?;
    let judged: Vec<FileDiff> = parse_diff(&diff).map_err(|error| {
        format!("weeder could not read the diff git produced: {error}. report it with the change that caused it.")
    })?;
    // The specimens leave here, before anything reads them: a file no rule
    // judges has no suppression to honour and no malformed one to complain
    // about either. What it has is a note, so the exclusion is in the log.
    let (judged, mut excluded) = set_aside(judged, &config.specimens);

    let mut changes = gather(&root, &range.base, &range.after, judged)?;
    if untracked {
        let (arrived, set_aside) = arrivals(&root, &config.specimens)?;
        changes.extend(arrived);
        excluded.extend(set_aside);
    }
    // Read after the change is whole, so an allowance written beside a line of
    // a file nobody staged counts the way one in a staged file does.
    let (inline, malformed) = parse_inline_suppressions(changes.iter().map(|change| &change.diff));
    let scope = scope(request, &config);
    // The specimens leave the repository's paths too: a rule that reads what the
    // tree holds, to resolve an import or to work out what a pattern hides,
    // reads the same repository every other rule was handed.
    let paths: Vec<String> = git::tracked_paths(&root)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|path| !specimen::skipped(&config.specimens, path))
        .collect();
    let callers = callers(&root, &changes, &scope)?;
    let hierarchy = hierarchy(&root, &range.after, &changes, &paths)?;
    // The settings are read once, here: the rule that judges what a line hides
    // and the refusal below both need the same reading, and reading them twice
    // would be two answers to one question.
    let collection = collection(&changes, &paths);
    let mut findings = rules::check::evaluate(&rules::check::Judgement {
        changes: &changes,
        config: &config,
        scope: &scope,
        paths: &paths,
        callers: &callers,
        hierarchy: &hierarchy,
        collection: &collection,
    });
    findings.extend(excluded.iter().map(|path| specimen::notice(path)));
    if request.strict {
        if let Some(unreadable) = malformed.first() {
            return Err(refusal(unreadable));
        }
        if let Some(unreadable) = collection.unreadable.first() {
            return Err(format!(
                "{} under --strict weeder will not judge a change whose collection it cannot read.",
                unreadable.complaint()
            ));
        }
    }

    // The person's allowance is read first, so where a trailer and an inline
    // marker name the same rule the finding carries the trailer: the two differ
    // in what a strict run does with them, and the one a strict run may honour
    // is the one the finding has to be holding.
    let mut suppressions = trailers(request, &root)?;
    suppressions.extend(inline);
    let findings = apply_suppressions(findings, &suppressions);
    // Under --strict a suppression is reported and not honoured: the finding
    // keeps the suppression it carries and gets its own level back, so a gate an
    // agent wrote an allowance for still stops it, and the reviewer sees both.
    // A caller that judges a change the message travels with says so, and the
    // one allowance a person could have written there is honoured; see
    // `honoured` below for which that is and why it is the only one.
    let findings: Vec<Finding> = if request.strict {
        findings
            .into_iter()
            .map(|mut finding| {
                if honoured(&finding, request) {
                    return finding;
                }
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

    Ok(Verdict {
        answer: Answer {
            code,
            stdout: write(&findings, request, &root),
            stderr: malformed
                .iter()
                .map(complaint)
                .chain(
                    collection
                        .unreadable
                        .iter()
                        .map(collect::Unreadable::complaint),
                )
                .collect(),
        },
        findings,
    })
}

/// Whether an allowance this run is honouring is what took the level off this
/// finding. Only a trailer can be one, and only where the caller judges a change
/// the message travels with; everything else gets the level its rule carries
/// back, so the reviewer sees the finding and what was said about it both.
fn honoured(finding: &Finding, request: &Request) -> bool {
    request.honour_trailers
        && finding
            .suppressed
            .as_ref()
            .is_some_and(|suppression| suppression.source == SuppressionSource::CommitTrailer)
}

/// The two states a run compares. The state a change starts from is always a
/// commit, git has nothing else to compare against, and what it ends in is
/// whichever of the three places the caller asked weeder to judge.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Range {
    base: String,
    after: Source,
}

/// What this run compares against what. A run that names two states weeder cannot
/// judge together stops here rather than guessing which one was meant.
fn range(request: &Request) -> Result<Range, String> {
    if request.staged && (request.base.is_some() || request.tip.is_some()) {
        return Err(
            "--base judges the tree against a ref and --staged judges the index, so weeder cannot do both. pass one."
                .to_string(),
        );
    }
    if request.base.is_none() && request.tip.is_some() {
        return Err(
            "a tip with no base names no range to judge, and weeder will not guess one."
                .to_string(),
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

/// Whether this run reads the files git has never been told about.
///
/// A caller who says nothing gets them wherever the working tree is what is
/// being judged against `HEAD`: that is plain `weeder check`, which is what a
/// stop hook runs and what pleach's smoke gate runs, and the whole of what a
/// worker produced is the question there. Everywhere else the answer is no,
/// because an index carries what a commit would carry and a range of commits
/// carries no working tree at all. Asking for them where there is no working
/// tree to read is a run that never happened rather than a flag that quietly
/// does nothing.
fn reads_untracked(request: &Request, range: &Range) -> Result<bool, String> {
    let tree = range.after == Source::Tree;
    match request.untracked {
        Some(Untracked::Exclude) => Ok(false),
        Some(Untracked::Include) if tree => Ok(true),
        Some(Untracked::Include) if request.staged => Err(
            "--staged judges the index, and a file git has never been told about is in no index, so --untracked include asks for something that is not there. pass one."
                .to_string(),
        ),
        Some(Untracked::Include) => Err(
            "a range of commits carries no working tree, so --untracked include has nothing to read there. judge the tree instead, or drop the flag."
                .to_string(),
        ),
        None => Ok(tree && range.base == "HEAD"),
    }
}

/// The files that arrived without git being told, each as the added file it is.
///
/// They are read here rather than asked of git, which will not diff a file it
/// has never heard of, and each one is read once: the same bytes become the
/// hunk a rule reads its lines from and the side a rule asks its weight and its
/// shape of. They arrive in the order the seam lists them, which is sorted, so
/// two runs over one tree write one log.
fn arrivals(root: &Path, specimens: &[String]) -> Result<(Vec<Change>, Vec<String>), String> {
    let mut arrived = Vec::new();
    let mut excluded = Vec::new();
    for path in git::untracked_files(root).map_err(|error| error.to_string())? {
        if specimen::skipped(specimens, &path) {
            excluded.push(path);
            continue;
        }
        // A file listed a moment ago and gone now is not a change anyone made;
        // it is a race with whoever deleted it, and there is nothing to judge.
        let Some(blob) = git::file_in_tree(root, &path).map_err(|error| error.to_string())? else {
            continue;
        };
        arrived.push(Change {
            diff: added_file(&path, blob.text().as_deref()),
            before: Side::default(),
            after: side(&path, &blob),
        });
    }
    Ok((arrived, excluded))
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

/// The changed files weeder judges, and the paths it was told to leave alone.
/// A specimen is skipped by every rule at once: the file never reaches a
/// detector, so no rule can be the one that read it anyway.
fn set_aside(judged: Vec<FileDiff>, specimens: &[String]) -> (Vec<FileDiff>, Vec<String>) {
    let mut kept = Vec::new();
    let mut excluded = Vec::new();
    for file in judged {
        match file
            .path()
            .filter(|path| specimen::skipped(specimens, path))
        {
            Some(path) => excluded.push(path.to_string()),
            None => kept.push(file),
        }
    }
    (kept, excluded)
}

/// The paths this run allows the change to touch. `--scope` is what the caller
/// asked for on this run; with no flag, whatever `weeder.toml` states, which is
/// everything until a repository says otherwise.
fn scope(request: &Request, config: &Config) -> Vec<String> {
    if request.scope.is_empty() {
        config.scope_globs.clone()
    } else {
        request.scope.clone()
    }
}

/// What this change does to what the runner collects. The whole repository is
/// read for its suites, the changed files for the settings that decide about
/// them, and what comes back is narrowed to the lines the change itself wrote.
fn collection(changes: &[Change], paths: &[String]) -> collect::Collection {
    let read: Vec<collect::File<'_>> = changes
        .iter()
        .filter_map(|change| {
            Some(collect::File {
                path: change.diff.new_path.as_deref()?,
                kind: change.after.kind(),
                lang: change.after.lang(),
                mask: change.after.mask(),
            })
        })
        .collect();
    collect::written_by(&collect::uncollected(&read, paths), changes)
}

/// Where the definitions of the out-of-scope files are called from. Only those
/// files are asked about: a walk of the tree is the one expensive thing a check
/// does, and with nothing outside the scope there is nothing to ask.
fn callers(root: &Path, changes: &[Change], scope: &[String]) -> Result<Vec<CallerSite>, String> {
    let symbols: BTreeSet<String> = changes
        .iter()
        .filter(|change| {
            change
                .path()
                .is_some_and(|path| !glob::matches_any(scope, path))
        })
        .flat_map(Change::changed_definitions)
        .collect();
    reader::callers(&symbols, root).map_err(|error| error.to_string())
}

/// What the tree is built out of, for the types the change left an unfinished
/// signal inside.
///
/// Reading a repository is the one expensive thing a check can do, so it happens
/// only where a rule has a question for it: a change that leaves no unfinished
/// signal inside a method reads nothing here at all. What is read is read from
/// where the change is being judged, so an override weeder credits is one the
/// same state carries, and only from the files written in a language whose
/// declarations say what a type is built on. A file whose bytes do not spell one
/// of the names asked about is passed over before the parser is asked anything,
/// which is what keeps the walk to the handful of files that can answer.
fn hierarchy(
    root: &Path,
    after: &Source,
    changes: &[Change],
    paths: &[String],
) -> Result<Hierarchy, String> {
    let wanted = rules::check::types_in_question(changes);
    if wanted.is_empty() {
        return Ok(Hierarchy::default());
    }
    // The index lists what a commit would carry; a file this run judges and git
    // has never been told about is in no listing and is read here all the same.
    let mut named: BTreeSet<&str> = paths.iter().map(String::as_str).collect();
    named.extend(
        changes
            .iter()
            .filter_map(|change| change.diff.new_path.as_deref()),
    );
    let named: Vec<&str> = named
        .into_iter()
        .filter(|path| hierarchy::states_its_bases(reader::language(Path::new(path))))
        .collect();

    let mut tree = Hierarchy::default();
    for (path, blob) in named.iter().zip(blobs(root, after, &named)?) {
        let Some(content) = blob.and_then(|blob| blob.text()) else {
            continue;
        };
        if !change::reads_as_code(Some(content.len() as u64))
            || !wanted.iter().any(|name| content.contains(name.as_str()))
        {
            continue;
        }
        let classification = classify_file(path, &content);
        let mask = Mask::of(classification.lang, &content);
        if hierarchy::is_entry(path) {
            for stated in hierarchy::stated_in(&wanted, &mask) {
                tree.exported
                    .entry(stated)
                    .or_insert_with(|| (*path).to_string());
            }
        }
        let outline = reader::outline(Path::new(path), &content);
        // A test file's own subclasses fill no contract in. A production file
        // that carries an inline test module is still a production file, and
        // what it declares is what the program is built out of; discounting the
        // whole of it for the module at the bottom would be the false block this
        // reading was written to take away.
        tree.derived.extend(hierarchy::derived_in(
            &wanted,
            classification.lang,
            classification.kind == FileKind::Test,
            &outline,
            &mask,
        ));
    }
    Ok(tree)
}

/// The `Weeder-allow:` trailers travelling with the change: the message handed in
/// with `--message-file`, and every commit a judged range carries. git's own
/// COMMIT_EDITMSG is never read: at pre-commit time it still holds the previous
/// commit's message, and a trailer must never outlive the change it was written for.
fn trailers(request: &Request, root: &Path) -> Result<Vec<Suppression>, String> {
    let mut messages = Vec::new();
    if let Some(path) = &request.message_file {
        messages.push(fs::read(path).map_err(|error| {
            format!("{error} --message-file must name a file weeder can read.")
        })?);
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

/// A `weeder-allow` with no reason gives weeder nothing to allow it against, so
/// under `--strict` the file goes unjudged rather than judged on a guess.
fn refusal(unreadable: &InlineSuppressionError) -> String {
    format!(
        "{} under --strict weeder will not judge a file whose suppression it cannot read.",
        complaint(unreadable)
    )
}

fn complaint(error: &InlineSuppressionError) -> String {
    format!(
        "{}:{} carries a {MARKER} weeder cannot read: {}. write it as `{MARKER} {}: the reason`.",
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
