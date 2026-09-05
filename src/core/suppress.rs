use crate::core::diff::{FileDiff, LineKind};
use crate::core::finding::{Finding, Level};

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
    message
        .lines()
        .filter_map(|line| {
            let trailer = line.trim().strip_prefix("Weed-allow:")?.trim();
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

pub fn parse_inline_suppressions(
    files: &[FileDiff],
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
                let Some(marker_index) = line.text.find("weed-allow") else {
                    continue;
                };
                let marker = line.text[marker_index + "weed-allow".len()..].trim();
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
