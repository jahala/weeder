//! The shapes production code is written in, where more than one rule reads them.
//!
//! S1 asks whether a function's body does anything. S2 asks the same of the
//! block a failure lands in. That is one question about two different spans of
//! lines, so where a block starts and stops, what a body holds once the
//! punctuation around it is cut away, and which statements amount to nothing,
//! are written here once and read from both.
//!
//! Everything is read through the syntax mask, so a brace inside a string never
//! opens a block and a colon inside a comment never starts a body.

use crate::core::classify::Lang;
use crate::core::syntax::Mask;

/// The statements that are a body doing nothing, whichever language wrote them.
pub const DO_NOTHING: &[&str] = &[
    "pass",
    "...",
    "return",
    "return null",
    "return nil",
    "return None",
    "return undefined",
];

/// The lines a block occupies, 1-based and inclusive at both ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    pub first: u32,
    pub last: u32,
}

/// The block a line opens, whichever way its language marks one out.
#[must_use]
pub fn block(lang: Lang, mask: &Mask, from: u32, after: usize) -> Option<Block> {
    match lang {
        Lang::Python => indented_block(mask, from),
        _ => braced_block(mask, from, after),
    }
}

/// The block a brace opens, counted from the character the caller points at.
///
/// The brace has to be on the line that opens the block. A language lets an
/// author put it on the next one, and weeder would then have to guess how far to
/// keep looking; a rule that guesses reports things nobody wrote.
fn braced_block(mask: &Mask, from: u32, after: usize) -> Option<Block> {
    let opening = mask
        .code(from)
        .chars()
        .skip(after)
        .position(|character| character == '{')?;
    let mut depth = 0_i32;
    for line in from..=mask.line_count() {
        let start = if line == from { after + opening } else { 0 };
        for character in mask.code(line).chars().skip(start) {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth <= 0 {
                        return Some(Block {
                            first: from,
                            last: line,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// The block an indent opens: everything under the line that is written further
/// in than it is. A blank line inside a block does not end it.
fn indented_block(mask: &Mask, from: u32) -> Option<Block> {
    let opening = indent(&written(mask, from))?;
    let mut last = from;
    for line in from.saturating_add(1)..=mask.line_count() {
        let text = written(mask, line);
        match indent(&text) {
            Some(deeper) if deeper > opening => last = line,
            Some(_) => break,
            None => continue,
        }
    }
    Some(Block { first: from, last })
}

/// What a block holds between the token that opens it and the one that closes
/// it. Comments are blanked and literals are not: a body whose only statement is
/// a comment holds no statements, and one that returns a literal returns something.
#[must_use]
pub fn body(lang: Lang, mask: &Mask, span: Block) -> Option<String> {
    let lines: Vec<String> = (span.first..=span.last)
        .map(|line| mask.outside_comments(line).into_owned())
        .collect();
    match lang {
        Lang::Python => indented_body(&lines),
        _ => braced_body(&lines),
    }
}

/// Whether a block says anything to the next reader.
#[must_use]
pub fn is_annotated(mask: &Mask, span: Block) -> bool {
    (span.first..=span.last).any(|line| !mask.comments(line).trim().is_empty())
}

/// The statements a body holds, each with its spacing and its terminator taken
/// off, so the same statement written two ways compares equal.
#[must_use]
pub fn statements(body: &str) -> Vec<String> {
    body.lines()
        .map(normalize)
        .filter(|statement| !statement.is_empty() && !is_punctuation(statement))
        .collect()
}

/// Whether a body does nothing at all: no statements, or none that do anything.
#[must_use]
pub fn does_nothing(statements: &[String]) -> bool {
    statements
        .iter()
        .all(|statement| DO_NOTHING.contains(&statement.as_str()))
}

/// One line with its spacing and its terminator taken off.
#[must_use]
pub fn normalize(line: &str) -> String {
    line.trim()
        .trim_end_matches(';')
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// A body that begins after the colon closing the declaration, whether the
/// statements follow on that line or under it.
fn indented_body(lines: &[String]) -> Option<String> {
    let mut depth = 0_i32;
    for (offset, line) in lines.iter().enumerate() {
        for (index, character) in line.char_indices() {
            match character {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ':' if depth <= 0 => {
                    let mut body = line[index + 1..].trim().to_string();
                    for later in &lines[offset + 1..] {
                        body.push('\n');
                        body.push_str(later);
                    }
                    return Some(body);
                }
                _ => {}
            }
        }
    }
    None
}

/// A body that begins at the brace opening it and ends at the one closing it.
fn braced_body(lines: &[String]) -> Option<String> {
    let text = lines.join("\n");
    let open = text.find('{')?;
    let close = text.rfind('}')?;
    (close > open).then(|| text[open + 1..close].to_string())
}

/// Whether a line is only the punctuation that holds a body together.
fn is_punctuation(statement: &str) -> bool {
    !statement.is_empty()
        && statement
            .chars()
            .all(|character| matches!(character, '{' | '}' | '(' | ')' | ',' | ':'))
}

/// The line as its author wrote it, comment and all, so a line that is only a
/// comment still has the indentation it was written at.
fn written(mask: &Mask, line: u32) -> String {
    let code = mask.outside_comments(line);
    if code.trim().is_empty() {
        mask.comments(line).into_owned()
    } else {
        code.into_owned()
    }
}

/// How far in a line is written, or `None` for a line with nothing on it.
fn indent(line: &str) -> Option<usize> {
    (!line.trim().is_empty()).then(|| line.len() - line.trim_start().len())
}
