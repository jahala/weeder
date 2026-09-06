//! S2, an error was swallowed.
//!
//! Four shapes, all of them the moment a program learns something went wrong
//! and forgets it again.
//!
//! A handler whose body does nothing: the block a failure lands in, `catch` or
//! `except`, holding no statements or only the language's word for none. A
//! comment inside the block is an answer, so a block that says why it is empty
//! is left alone.
//!
//! A failure tested against the language's word for nothing, and a block that
//! answers by returning nothing. Go writes this one, and it is read narrowly on
//! purpose: the nil check is Go's ordinary punctuation, and only a block with
//! nothing in it is a failure going nowhere.
//!
//! A failure assigned to the name that means "throw this away". Every language
//! weed reads spells that `_`.
//!
//! A fallible call turned into a value that cannot fail, with nothing asked
//! about what went wrong.

use crate::core::change::Change;
use crate::core::classify::{FileKind, Lang};
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::rules::check::idiom;
use crate::core::rules::check::vocab::mentions_failure;
use crate::core::rules::check::Judgement;
use crate::core::syntax::{words, Mask, Word};

/// The name every language weed reads gives a binding nobody will look at.
const DISCARD: &str = "_";

/// How a language spells the moment a failure is handled.
struct Handling {
    /// The keyword that opens the block a failure lands in.
    handler: Option<&'static str>,
    /// The language's word for nothing, where a failure is handled by being
    /// compared against it.
    nothing: Option<&'static str>,
    /// Members that turn a failure into a value without reading it.
    discarding: &'static [&'static str],
}

const NOTHING_HANDLED: Handling = Handling {
    handler: None,
    nothing: None,
    discarding: &[],
};

fn handling(lang: Lang) -> Handling {
    match lang {
        Lang::TypeScript | Lang::JavaScript => Handling {
            handler: Some("catch"),
            ..NOTHING_HANDLED
        },
        Lang::Python => Handling {
            handler: Some("except"),
            ..NOTHING_HANDLED
        },
        // Rust hands a failure back rather than catching it, so what swallows
        // one is a call that turns it into a value with nothing asked.
        Lang::Rust => Handling {
            discarding: &["ok", "unwrap_or_default"],
            ..NOTHING_HANDLED
        },
        Lang::Go => Handling {
            nothing: Some("nil"),
            ..NOTHING_HANDLED
        },
        Lang::Other => NOTHING_HANDLED,
    }
}

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let mut findings = Vec::new();
    for change in judged.changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !change.after.is(FileKind::Prod) {
            continue;
        }
        let lang = lang(change);
        let mask = change.after.mask();
        let table = handling(lang);
        for (line, _) in change.added() {
            if let Some(swallowed) = swallowed(&table, lang, mask, line) {
                findings.push(finding(path, line, &swallowed));
            }
        }
    }
    findings
}

/// What an added line swallows, or nothing where it swallows nothing.
fn swallowed(table: &Handling, lang: Lang, mask: &Mask, line: u32) -> Option<Swallowed> {
    let code = mask.code(line);
    let spoken = words(&code);
    if let Some(keyword) = table.handler {
        if let Some(word) = spoken.iter().find(|word| word.text == keyword) {
            if is_hollow(lang, mask, line, characters(&code, word.end)) {
                return Some(Swallowed::handler(keyword));
            }
        }
    }
    if let Some(nothing) = table.nothing {
        if let Some(at) = nothing_check(&code, nothing) {
            if is_hollow(lang, mask, line, characters(&code, at)) {
                return Some(Swallowed::nothing_check(nothing));
            }
        }
    }
    if let Some(word) = spoken.iter().find(|word| discards(&code, word)) {
        return Some(Swallowed::discarded(word.text));
    }
    let converted = spoken.iter().find(|word| {
        word.is_member() && word.is_called() && table.discarding.contains(&word.text)
    })?;
    Some(Swallowed::converted(converted.text))
}

/// Whether the block opened at `after` on this line holds nothing that does
/// anything, and says nothing about why.
fn is_hollow(lang: Lang, mask: &Mask, line: u32, after: usize) -> bool {
    let Some(span) = idiom::block(lang, mask, line, after) else {
        return false;
    };
    if idiom::is_annotated(mask, span) {
        return false;
    }
    let Some(body) = idiom::body(lang, mask, span) else {
        return false;
    };
    idiom::does_nothing(&idiom::statements(&body))
}

/// Where a line tests a failure against the language's word for nothing, as the
/// byte the block after it opens at. A comparison that names no failure, or one
/// that is not an inequality, is the language's ordinary punctuation. A line may
/// hold several comparisons, and the one this rule is about is whichever of them
/// puts a failure on the left and nothing on the right.
fn nothing_check(code: &str, nothing: &str) -> Option<usize> {
    code.match_indices("!=")
        .filter(|(at, _)| {
            code[at + 2..]
                .split_whitespace()
                .next()
                .is_some_and(|word| word.trim_end_matches('{') == nothing)
        })
        .map(|(at, _)| at)
        .find(|at| mentions_failure(&code[..*at]))
}

/// Whether a word is the discard binding taking a failure. The binding has to
/// open a statement of its own, so a `_` standing among the values a call hands
/// back is not this shape; Go spells the assignment with a colon in front of the
/// equals, and both spellings are the same act.
fn discards(code: &str, word: &Word<'_>) -> bool {
    if word.text != DISCARD || !matches!(word.before, None | Some(';') | Some('{')) {
        return false;
    }
    if !matches!(word.after, Some('=') | Some(':')) {
        return false;
    }
    let assigned = code[word.end..].trim_start_matches([':', '=', ' ']);
    mentions_failure(assigned)
}

/// How many characters of a line come before a byte offset. A block is counted
/// in characters, because that is what the mask holds.
fn characters(code: &str, at: usize) -> usize {
    code[..at].chars().count()
}

fn lang(change: &Change) -> Lang {
    change
        .after
        .classification
        .as_ref()
        .map_or(Lang::Other, |classification| classification.lang)
}

/// A swallowed failure, as it will be put to the reader.
struct Swallowed {
    what: String,
    why: &'static str,
}

impl Swallowed {
    fn handler(keyword: &str) -> Swallowed {
        Swallowed {
            what: format!("a `{keyword}` was added whose block does nothing with the failure."),
            why: "the program learns the call went wrong and carries on as though it had not.",
        }
    }

    fn nothing_check(nothing: &str) -> Swallowed {
        Swallowed {
            what: format!("a failure was tested for and answered with {nothing}."),
            why:
                "the caller is told everything worked, and the failure stops here without a trace.",
        }
    }

    fn discarded(name: &str) -> Swallowed {
        Swallowed {
            what: format!("a failure was assigned to `{name}`, the name for a value nobody reads."),
            why: "the compiler stops asking about the error, and so does everyone else.",
        }
    }

    fn converted(call: &str) -> Swallowed {
        Swallowed {
            what: format!("`{call}` turns a failure into a value with nothing asked about it."),
            why: "what went wrong is gone by the next line, and the default it left behind reads like a result.",
        }
    }
}

fn finding(path: &str, line: u32, swallowed: &Swallowed) -> Finding {
    Finding {
        rule: "S2".to_string(),
        level: Level::Warn,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: swallowed.what.clone(),
            why: swallowed.why.to_string(),
            next: "log the failure, wrap it for the caller, hand it on, or say in a comment why it is right to drop it.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
