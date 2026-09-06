//! C2, an ignore file was broadened over source or tests.
//!
//! An ignore file decides what the tools and the reviewer are shown. A pattern
//! added there that covers the repository's own source or its suite takes work
//! out of sight without touching a line of it, and the next diff looks clean
//! because half of it is no longer being read.
//!
//! Whether a pattern covers source is not a question about the pattern, it is a
//! question about this repository: the pattern is turned into path globs the way
//! the ignore files spell them, matched against the paths the repository holds,
//! and what it catches is classified. A pattern that catches nothing, or catches
//! only what a build wrote, is an ignore file doing its job.

use crate::core::classify::{classify_file, FileKind};
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::glob;
use crate::core::rules::check::Judgement;

/// The files a repository states what to look away from in.
const IGNORE_FILES: &[&str] = &[
    ".gitignore",
    ".eslintignore",
    ".prettierignore",
    ".dockerignore",
];

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let mut findings = Vec::new();
    for change in judged.changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !is_ignore_file(path) {
            continue;
        }
        for (line, text) in change.added() {
            let Some(pattern) = pattern(text) else {
                continue;
            };
            let hidden = hidden_by(pattern, directory(path), judged.paths);
            if hidden.is_empty() {
                continue;
            }
            findings.push(finding(path, line, pattern, &hidden));
        }
    }
    findings
}

fn is_ignore_file(path: &str) -> bool {
    IGNORE_FILES.contains(&path.rsplit('/').next().unwrap_or(path))
}

/// The pattern an added line states, or nothing where the line states none. A
/// line that takes a pattern back out, which every ignore file spells with a
/// `!`, is narrowing rather than broadening.
fn pattern(text: &str) -> Option<&str> {
    let pattern = text.trim();
    (!pattern.is_empty() && !pattern.starts_with('#') && !pattern.starts_with('!'))
        .then_some(pattern)
}

/// The source and test files in the repository that a pattern would cover.
fn hidden_by(pattern: &str, base: &str, paths: &[String]) -> Vec<String> {
    let globs = globs(pattern, base);
    paths
        .iter()
        .filter(|path| glob::matches_any(&globs, path))
        .filter(|path| {
            matches!(
                classify_file(path, "").kind,
                FileKind::Prod | FileKind::Test
            )
        })
        .cloned()
        .collect()
}

/// A pattern as the path globs it covers. A pattern that ends in a separator
/// covers a directory and nothing else; one that holds a separator anywhere else
/// is read from the ignore file's own directory; one that holds none at all
/// matches by name, at any depth.
fn globs(pattern: &str, base: &str) -> Vec<String> {
    let directory_only = pattern.ends_with('/');
    let trimmed = pattern.trim_end_matches('/');
    let anchored = trimmed.starts_with('/') || trimmed.trim_start_matches('/').contains('/');
    let named = trimmed.trim_start_matches('/');
    let body = if anchored {
        format!("{base}{named}")
    } else {
        format!("{base}**/{named}")
    };
    let inside = format!("{body}/**");
    if directory_only {
        vec![inside]
    } else {
        vec![body, inside]
    }
}

/// The directory an ignore file states its patterns from, ending in a separator
/// or empty at the root of the repository.
fn directory(path: &str) -> &str {
    match path.rfind('/') {
        Some(at) => &path[..=at],
        None => "",
    }
}

fn finding(path: &str, line: u32, pattern: &str, hidden: &[String]) -> Finding {
    let first = hidden.first().map(String::as_str).unwrap_or_default();
    let rest = hidden.len().saturating_sub(1);
    Finding {
        rule: "C2".to_string(),
        level: Level::Warn,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!("`{pattern}` was added to an ignore file, and it covers {first}."),
            why: match rest {
                0 => "code that is ignored is code nobody reviews and no tool reads.".to_string(),
                _ => format!("it covers {rest} more of this repository's own files, and code that is ignored is code nobody reviews."),
            },
            next: "narrow the pattern to what a build wrote, or take it back out.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
