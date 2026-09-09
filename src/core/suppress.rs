//! The tokens are `Weeder-allow:` in a commit message and `weeder-allow <RULE>:`
//! beside a line, and the gate reads those and nothing else.
//!
//! They carried the tool's older name for two days after the binary stopped
//! carrying it, and the reason was the evidence rather than sentiment: the
//! finding messages that teach the tokens were printed into the blind audit's
//! case packets, each recorded auditor answer opened with the SHA-256 of the
//! packet it was given, and rewording a message broke every one of those seals.
//! The rename was made anyway, on the owner's ruling of 2026-09-08, and it cost
//! what it was always going to cost: forty fresh auditor sessions, blind and
//! sighted, on regenerated packets.
//!
//! It cost that once. The seal no longer covers wording. A packet's hash is
//! taken over the judged content, the rule ids, each finding's level, path and
//! line, and the hunks the auditor reads, because the question an auditor
//! answers is whether the rule's claim is true of the change, and no sentence
//! weeder writes about what to do next changes that answer. So a message here
//! can be rewritten for a reader without buying forty more sessions, and the
//! next person to improve one of these sentences owes nobody anything.
//!
//! Reading the retired spelling as well would be the shim weeder refuses in
//! other people's diffs. A history read is not the gate, though, and
//! `cargo xtask suppressions` counts both spellings when it measures how often
//! a repository wrote itself an allowance: what was written under the old name
//! was still an allowance, and leaving it out would flatter the rate.
use crate::core::diff::{FileDiff, LineKind};
use crate::core::finding::{Finding, Level};

/// What an allowance is called in a commit message.
pub const TRAILER: &str = "Weeder-allow:";

/// What it is called beside a line. It is spelled the way code is written,
/// because that is where it is written.
pub const MARKER: &str = "weeder-allow";

/// The trailer the tool carried before it was renamed. It suppresses nothing:
/// the one place it is read is a count of what a repository has written over
/// its history, where leaving it out would say a repository allowed less past
/// than it did. Nothing on the gate's path may name this.
const RETIRED_TRAILER: &str = "Weed-allow:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuppressionSource {
    CommitTrailer,
    InlineComment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suppression {
    pub rule: String,
    pub reason: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub source: SuppressionSource,
    pub original_level: Option<Level>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSuppressionError {
    pub rule: String,
    pub path: String,
    pub line: u32,
    pub message: String,
}

pub fn parse_commit_suppressions(message: &str) -> Vec<Suppression> {
    trailers(message, &[TRAILER])
}

/// The allowances a commit message carries, counted rather than honoured: both
/// spellings, because a message written before the rename allowed something
/// past a gate that was running, and a measurement that skipped those would
/// report a repository as more obedient than it was. Nothing that judges a
/// change calls this.
#[must_use]
pub fn parse_history_suppressions(message: &str) -> Vec<Suppression> {
    trailers(message, &[TRAILER, RETIRED_TRAILER])
}

fn trailers(message: &str, tokens: &[&str]) -> Vec<Suppression> {
    message
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let trailer = tokens
                .iter()
                .find_map(|token| line.strip_prefix(token))?
                .trim();
            let (rule, reason) = split_rule_reason(trailer)?;
            Some(Suppression {
                rule,
                reason,
                path: None,
                line: None,
                source: SuppressionSource::CommitTrailer,
                original_level: None,
            })
        })
        .collect()
}

/// Every allowance written beside a line of the change, and every one weeder
/// could not read. It is handed the changed files rather than a list, because
/// a face builds that list out of more than one question to git.
pub fn parse_inline_suppressions<'a>(
    files: impl IntoIterator<Item = &'a FileDiff>,
) -> (Vec<Suppression>, Vec<InlineSuppressionError>) {
    let mut suppressions = Vec::new();
    let mut errors = Vec::new();
    for file in files {
        let path = file
            .new_path
            .as_ref()
            .or(file.old_path.as_ref())
            .cloned()
            .unwrap_or_default();
        for hunk in &file.hunks {
            for line in &hunk.lines {
                if !matches!(line.kind, LineKind::Added | LineKind::Context) {
                    continue;
                }
                let Some(marker_index) = line.text.find(MARKER) else {
                    continue;
                };
                let rest = &line.text[marker_index + MARKER.len()..];
                if !writes_an_allowance(rest) {
                    continue;
                }
                let marker = rest.trim();
                let reported_line = line.new_line.or(line.old_line).unwrap_or(0);
                match parse_inline_marker(marker) {
                    Ok((rule, reason)) => suppressions.push(Suppression {
                        rule,
                        reason,
                        path: Some(path.clone()),
                        line: Some(reported_line),
                        source: SuppressionSource::InlineComment,
                        original_level: None,
                    }),
                    Err(rule) => errors.push(InlineSuppressionError {
                        rule,
                        path: path.clone(),
                        line: reported_line,
                        message: "inline suppression needs a reason".to_string(),
                    }),
                }
            }
        }
    }
    (suppressions, errors)
}

pub fn apply_suppressions(findings: Vec<Finding>, suppressions: &[Suppression]) -> Vec<Finding> {
    findings
        .into_iter()
        .map(|mut finding| {
            if let Some(suppression) = suppressions
                .iter()
                .find(|suppression| suppression_matches(&finding, suppression))
            {
                let mut applied = suppression.clone();
                applied.original_level = Some(finding.level);
                finding.level = Level::Note;
                finding.suppressed = Some(applied);
            }
            finding
        })
        .collect()
}

fn suppression_matches(finding: &Finding, suppression: &Suppression) -> bool {
    if suppression.rule != finding.rule {
        return false;
    }
    if let Some(path) = &suppression.path {
        if path != &finding.path {
            return false;
        }
    }
    if let (Some(line), Some(region)) = (suppression.line, &finding.region) {
        return line >= region.start_line.saturating_sub(1) && line <= region.end_line;
    }
    true
}

/// Whether a line that names the marker is writing one, read from what follows
/// it: the rule id an allowance is for, or the colon of one somebody left out.
///
/// Anything else is the word appearing in a sentence. weeder's own pages explain
/// the token, its own source holds it in a constant, and a repository that
/// documents its allowances writes the marker in prose the way this paragraph
/// does. A judge that read every one of those as a broken allowance would
/// refuse, under `--strict`, to judge the page that teaches it, and a rule whose
/// documentation stops the gate is a rule nobody documents.
fn writes_an_allowance(rest: &str) -> bool {
    if rest.starts_with(':') {
        return true;
    }
    if !rest.starts_with(char::is_whitespace) {
        return false;
    }
    let named = rest.trim_start();
    let rule = named
        .find(|character: char| !character.is_ascii_alphanumeric())
        .unwrap_or(named.len());
    if rule == 0 {
        return false;
    }
    // A rule id runs to the colon that opens its reason, or to the end of what
    // the line says. A quote, a bracket or a backslash against it is a line
    // writing about the marker rather than with it.
    match named[rule..].chars().next() {
        None | Some(':') => true,
        Some(character) => character.is_whitespace(),
    }
}

fn parse_inline_marker(marker: &str) -> Result<(String, String), String> {
    let Some((rule, reason)) = marker.split_once(':') else {
        let rule = marker
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string();
        return Err(rule);
    };
    let rule = rule.trim().to_string();
    let reason = reason.trim().to_string();
    if rule.is_empty() || reason.is_empty() {
        Err(rule)
    } else {
        Ok((rule, reason))
    }
}

fn split_rule_reason(text: &str) -> Option<(String, String)> {
    let mut parts = text.splitn(2, char::is_whitespace);
    let rule = parts.next()?.trim();
    let reason = parts.next()?.trim();
    if rule.is_empty() || reason.is_empty() {
        None
    } else {
        Some((rule.to_string(), reason.to_string()))
    }
}
