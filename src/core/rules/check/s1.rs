//! S1, a stub or a work marker reached production code.
//!
//! Three shapes, all of them a line admitting the work is not done.
//!
//! A work marker, one of the shouted words in [`WORK_MARKERS`], written as a
//! word of its own, in code or in a comment but never inside a string, where it
//! is a message the program hands out rather than a note to the author.
//!
//! An unfinished signal: the token a language keeps for a body nobody wrote
//! (`todo!`, `unimplemented!`, `NotImplementedError`), or a throw, a raise or a
//! panic whose message, read as letters alone, says the same thing.
//!
//! A body that does nothing: a function whose whole body is the language's word
//! for "no statements", `pass`, `...`, a bare return, a return of the language's
//! empty value. The reader's outline says where each body starts and ends, the
//! change says the body is new, and the finding lands on the statement itself.

use crate::core::change::Change;
use crate::core::classify::{FileKind, Lang};
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::read::{Definition, DefinitionKind};
use crate::core::syntax::{words, Mask};

/// The words that mark work left for later, written as they are meant to be
/// found: in capitals, so the same word in a sentence is prose and the shouted
/// one is a marker. weed does not carry what it refuses, which is why these
/// three are written here and nowhere else in this repository.
const WORK_MARKERS: &[&str] = &["TODO", "FIXME", "XXX"];

/// The tokens a language keeps for a body nobody wrote. `todo` counts only as a
/// macro, because it is also an ordinary name for an ordinary thing.
const UNFINISHED_TOKENS: &[&str] = &["unimplemented", "NotImplemented", "NotImplementedError"];
const UNFINISHED_MACRO: &str = "todo";

/// The keywords that hand a failure to the caller rather than returning one.
/// A message given to one of these is the code describing itself.
const RAISING: &[&str] = &["throw", "raise", "panic"];

/// A message that says the work is not done, with everything but its letters
/// taken out, so spelling and punctuation stop mattering.
const UNFINISHED_PHRASES: &[&str] = &["notimplemented", "notyetimplemented", "unimplemented"];

/// The statements that are a body doing nothing, whichever language wrote them.
const DO_NOTHING: &[&str] = &[
    "pass",
    "...",
    "return",
    "return null",
    "return nil",
    "return None",
    "return undefined",
];

pub fn evaluate(changes: &[Change]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for change in changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !change.after.is(FileKind::Prod) {
            continue;
        }
        let lang = change
            .after
            .classification
            .as_ref()
            .map_or(Lang::Other, |classification| classification.lang);
        let mask = change.after.mask();
        let mut stubs: Vec<(u32, Stub)> = change
            .added()
            .filter_map(|(line, _)| Some((line, stub_on(&mask, line)?)))
            .collect();
        // A body that does nothing is found through the outline rather than
        // line by line, so it arrives out of order and may land on a line a
        // marker already spoke for.
        for (line, stub) in empty_bodies(lang, &mask, change) {
            if !stubs.iter().any(|(reported, _)| *reported == line) {
                stubs.push((line, stub));
            }
        }
        stubs.sort_by_key(|(line, _)| *line);
        findings.extend(
            stubs
                .into_iter()
                .map(|(line, stub)| finding(path, line, &stub)),
        );
    }
    findings
}

/// The stub an added line carries on its own, without the outline's help.
fn stub_on(mask: &Mask, line: u32) -> Option<Stub> {
    let outside = mask.outside_literals(line);
    if let Some(word) = words(&outside)
        .into_iter()
        .find(|word| WORK_MARKERS.contains(&word.text))
    {
        return Some(Stub::marker(word.text));
    }

    let code = mask.code(line);
    let spoken = words(&code);
    if let Some(word) = spoken.iter().find(|word| {
        (UNFINISHED_TOKENS.contains(&word.text) && !word.is_member())
            || (word.text == UNFINISHED_MACRO && word.after == Some('!'))
    }) {
        // A macro is quoted back with the `!` that makes it one, so the reader
        // is shown the thing they wrote rather than the name inside it.
        let written = match word.after {
            Some('!') => format!("{}!", word.text),
            _ => word.text.to_string(),
        };
        return Some(Stub::token(&written));
    }

    let raising = spoken.iter().any(|word| RAISING.contains(&word.text));
    if raising && says_unfinished(&mask.literals(line)) {
        return Some(Stub::announcement());
    }
    None
}

/// Whether a message, read as letters alone, says the work is not done.
fn says_unfinished(text: &str) -> bool {
    let letters: String = text
        .chars()
        .filter(|character| character.is_alphabetic())
        .flat_map(char::to_lowercase)
        .collect();
    UNFINISHED_PHRASES
        .iter()
        .any(|phrase| letters.contains(phrase))
}

/// Every function the change added a line to whose whole body does nothing,
/// with the added line the finding lands on.
fn empty_bodies(lang: Lang, mask: &Mask, change: &Change) -> Vec<(u32, Stub)> {
    let added: Vec<u32> = change.added().map(|(line, _)| line).collect();
    let mut found = Vec::new();
    for definition in change.after.outline.flatten() {
        if definition.kind != DefinitionKind::Function {
            continue;
        }
        let Some((line, statement)) = does_nothing(lang, mask, definition) else {
            continue;
        };
        if !added.iter().any(|added| definition.spans(*added)) {
            continue;
        }
        found.push((line, Stub::empty_body(&definition.name, &statement)));
    }
    found
}

/// The one statement a function's body holds and the line it is written on,
/// where that statement does nothing at all. A body with anything else in it
/// answers `None`.
fn does_nothing(lang: Lang, mask: &Mask, definition: &Definition) -> Option<(u32, String)> {
    let body = body(lang, mask, definition)?;
    let statements: Vec<String> = body
        .lines()
        .map(normalize)
        .filter(|statement| !statement.is_empty() && !is_punctuation(statement))
        .collect();
    let [only] = statements.as_slice() else {
        return None;
    };
    if !DO_NOTHING.contains(&only.as_str()) {
        return None;
    }
    // Where the statement is written on a line of its own, that is the line
    // the finding points at; a body written on the declaration's own line has
    // nowhere else to point than the declaration.
    let line = (definition.start_line..=definition.end_line)
        .find(|line| normalize(&mask.outside_comments(*line)) == *only)
        .unwrap_or(definition.start_line);
    Some((line, only.to_string()))
}

/// What a definition writes between the start of its body and the end of it.
///
/// Comments are blanked and literals are not: a body whose only statement is a
/// comment is a body that does nothing, and a body that returns a literal is a
/// body that returns something.
fn body(lang: Lang, mask: &Mask, definition: &Definition) -> Option<String> {
    let lines: Vec<String> = (definition.start_line..=definition.end_line)
        .map(|line| mask.outside_comments(line))
        .collect();
    match lang {
        Lang::Python => indented_body(&lines),
        _ => braced_body(&lines),
    }
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

/// One line of a body with its spacing and its terminator taken off, so the same
/// statement written two ways compares equal.
fn normalize(line: &str) -> String {
    line.trim()
        .trim_end_matches(';')
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Whether a line is only the punctuation that holds a body together.
fn is_punctuation(statement: &str) -> bool {
    !statement.is_empty()
        && statement
            .chars()
            .all(|character| matches!(character, '{' | '}' | '(' | ')' | ',' | ':'))
}

/// A stub, as it will be quoted back to the reader.
struct Stub {
    what: String,
    why: &'static str,
}

impl Stub {
    fn marker(word: &str) -> Stub {
        Stub {
            what: format!("a work marker reached production code: `{word}`."),
            why: "a marker is a note to the author, and it ships as behaviour nobody wrote.",
        }
    }

    fn token(word: &str) -> Stub {
        Stub {
            what: format!("an unwritten body reached production code: `{word}`."),
            why: "the line compiles and fails at the moment a caller reaches it.",
        }
    }

    fn announcement() -> Stub {
        Stub {
            what: "production code was added that raises to say it is not implemented.".to_string(),
            why: "the line compiles and fails at the moment a caller reaches it.",
        }
    }

    fn empty_body(name: &str, statement: &str) -> Stub {
        Stub {
            what: format!("`{name}` was added with a body that does nothing: `{statement}`."),
            why: "a function that answers without working is a promise the caller cannot tell from a result.",
        }
    }
}

fn finding(path: &str, line: u32, stub: &Stub) -> Finding {
    Finding {
        rule: "S1".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: stub.what.clone(),
            why: stub.why.to_string(),
            next: "finish the work, or take the line back out of the change.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
