//! G1 — a conflict marker was committed.
//!
//! git writes four markers into a conflicted file, each exactly seven of its
//! character at the start of a line, alone or followed by a space and a label.
//! `<<<<<<<` and `>>>>>>>` mean nothing else in any language weed reads, so they
//! are reported wherever they are added. `=======` and `|||||||` are also a
//! setext heading rule, a comment divider and a table rule, so weed reports them
//! only in a file whose new content carries an opener or a closer as well: a
//! separator with no conflict around it is punctuation, not a conflict.

use crate::core::diff::{FileDiff, LineKind};
use crate::core::finding::{Finding, Fix, Level, Message, Region};

/// How many times a marker repeats its character.
const WIDTH: usize = 7;
const OURS: char = '<';
const BASE: char = '|';
const SEPARATOR: char = '=';
const THEIRS: char = '>';

pub fn evaluate(files: &[FileDiff]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in files {
        let Some(path) = file.new_path.as_deref() else {
            continue;
        };
        let conflicted = new_content(file).any(|text| marker(text).is_some_and(is_unambiguous));
        for hunk in &file.hunks {
            for line in &hunk.lines {
                if line.kind != LineKind::Added {
                    continue;
                }
                let Some(found) = marker(&line.text) else {
                    continue;
                };
                if !is_unambiguous(found) && !conflicted {
                    continue;
                }
                let Some(start_line) = line.new_line else {
                    continue;
                };
                findings.push(finding(path, start_line, found));
            }
        }
    }
    findings
}

/// The marker a line starts with, if it starts with one.
fn marker(line: &str) -> Option<char> {
    let first = line.chars().next()?;
    if !matches!(first, OURS | BASE | SEPARATOR | THEIRS) {
        return None;
    }
    if line
        .chars()
        .take_while(|character| *character == first)
        .count()
        != WIDTH
    {
        return None;
    }
    // The markers are ASCII, so seven of them end on a character boundary.
    match line[WIDTH..].chars().next() {
        None | Some(' ') => Some(first),
        Some(_) => None,
    }
}

fn is_unambiguous(marker: char) -> bool {
    matches!(marker, OURS | THEIRS)
}

/// Every line the change leaves in the file: what it added, and the context it
/// kept. A conflict opened before the hunk still shows here as context.
fn new_content(file: &FileDiff) -> impl Iterator<Item = &str> {
    file.hunks
        .iter()
        .flat_map(|hunk| hunk.lines.iter())
        .filter(|line| matches!(line.kind, LineKind::Added | LineKind::Context))
        .map(|line| line.text.as_str())
}

fn finding(path: &str, start_line: u32, found: char) -> Finding {
    let marker: String = std::iter::repeat_n(found, WIDTH).collect();
    Finding {
        rule: "G1".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: Some(Region {
            start_line,
            end_line: start_line,
        }),
        message: Message {
            what: format!("a merge conflict marker ({marker}) was added."),
            why: "the file carries both sides of a merge nobody finished, so it does not parse and does not run.".to_string(),
            next: "finish the merge, delete the markers, and stage the file again.".to_string(),
        },
        fix: Some(Fix {
            description: "delete the conflict marker line.".to_string(),
            replacement: None,
        }),
        suppressed: None,
    }
}
