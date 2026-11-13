//! G3, a control character was added to a text source file.
//!
//! git takes a file for binary once it holds one NUL byte, and from then on
//! every change to it reads `Binary files differ` with no line anybody can
//! review. A NUL in a source file has no honest reading: a language writes the
//! character as an escape, and the raw byte is what an editor or a tool leaves
//! behind where a character was meant. So a NUL blocks. Every other control
//! character, a form feed, an escape, a stray `0x01`, prints as nothing or moves
//! the cursor, and some have an old honest use, the form feed as a page break
//! above all, so those warn.
//!
//! weeder asks git for the text of every change, so the lines of such a file
//! arrive as hunks even where git would show none, and a finding names the
//! added line the character sits on. A character the change took out on one
//! line and put back on another, the way an edit beside it does, was in the
//! file before and is not counted again: what is reported is what the file
//! holds more of than it did.

use std::collections::BTreeMap;

use crate::core::change::Change;
use crate::core::classify::reads_as_text;
use crate::core::finding::{Detection, Level, Message, Region, Stamp};
use crate::core::rules::check::Judgement;

const NUL: char = '\u{0}';

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let mut findings = Vec::new();
    for change in judged.changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !is_text_source(path, change) {
            continue;
        }
        let gained = gained(change);
        for (line, text) in change.added() {
            let mut seen: Vec<char> = Vec::new();
            for character in text.chars().filter(|character| is_reported(*character)) {
                if seen.contains(&character) || gained.get(&character).is_none_or(|net| *net <= 0) {
                    continue;
                }
                seen.push(character);
                findings.push(finding(path, line, character));
            }
        }
    }
    findings
}

/// Whether the file is one whose lines a person is meant to read: its path says
/// it is text, whatever its bytes hold, or git counted its base as text.
fn is_text_source(path: &str, change: &Change) -> bool {
    reads_as_text(path) || (change.before.size.is_some() && !change.before.binary)
}

/// A control character other than the three every text file is made of.
fn is_reported(character: char) -> bool {
    character.is_ascii_control() && !matches!(character, '\t' | '\n' | '\r')
}

/// How many more of each control character the change put in than it took out.
/// The lines it kept are the same on both sides, so this is how many more the
/// file holds than it did.
fn gained(change: &Change) -> BTreeMap<char, i64> {
    change.gained(|_, _, text| {
        text.chars()
            .filter(|character| is_reported(*character))
            .collect()
    })
}

/// What a control character is called, in the words of the Unicode standard.
fn name(character: char) -> &'static str {
    match u32::from(character) {
        0x00 => "null",
        0x01 => "start of heading",
        0x02 => "start of text",
        0x03 => "end of text",
        0x04 => "end of transmission",
        0x05 => "enquiry",
        0x06 => "acknowledge",
        0x07 => "bell",
        0x08 => "backspace",
        0x0b => "line tabulation",
        0x0c => "form feed",
        0x0e => "shift out",
        0x0f => "shift in",
        0x10 => "data link escape",
        0x11 => "device control one",
        0x12 => "device control two",
        0x13 => "device control three",
        0x14 => "device control four",
        0x15 => "negative acknowledge",
        0x16 => "synchronous idle",
        0x17 => "end of transmission block",
        0x18 => "cancel",
        0x19 => "end of medium",
        0x1a => "substitute",
        0x1b => "escape",
        0x1c => "file separator",
        0x1d => "group separator",
        0x1e => "record separator",
        0x1f => "unit separator",
        0x7f => "delete",
        _ => "control character",
    }
}

fn finding(path: &str, line: u32, character: char) -> Detection {
    let point = format!("U+{:04X}", u32::from(character));
    let (stamp, why, next) = if character == NUL {
        (
            Stamp::Rule,
            "git now takes the whole file for binary and writes `Binary files differ` in place of every change to it, so no reviewer sees its lines again.".to_string(),
            "delete the byte or write the character it stood in for, or write why it stays in a `Weeder-allow: G3` trailer.".to_string(),
        )
    } else {
        (
            Stamp::Override(Level::Warn),
            "it prints as nothing or moves the cursor, so the line a reviewer reads is not the line the file holds.".to_string(),
            "delete it, or write it as the escape the language reads where the program needs the character.".to_string(),
        )
    };
    Detection {
        stamp,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!(
                "a control character, {point} ({}), was added to this line.",
                name(character)
            ),
            why,
            next,
        },
        fix: None,
    }
}
