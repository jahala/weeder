//! C1, a guardrail was edited.
//!
//! A guardrail is a file that decides what the other checks do: the workflow
//! that runs them, the settings an agent harness reads, the hook git calls,
//! weeder's own law. A change there can turn every gate off without touching a
//! single test, which is why any change to one is the finding, added, edited or
//! taken away, and whatever it says.
//!
//! One file under `.githooks/` is not an edit: the bundle weeder itself writes,
//! byte for byte, for the binary and the branches it names. Adopting weeder is a
//! change the gate lets through, because weeder can tell its own hook from one
//! somebody rewrote; a hook that differs by one byte is a guardrail edit again.
//!
//! The two markdown files are different. `AGENTS.md` and `CLAUDE.md` are mostly
//! prose that wants editing, and one section of them is law. So weeder reads the
//! headings, finds the sections that state the law, and reports only a hunk
//! that lands inside one. A paragraph rewritten three sections down is writing;
//! a line changed under the limits is a rule being rewritten.
//!
//! A repository names its law in its own words. weeder knows the ones such
//! headings are written in everywhere, [`LAW_HEADINGS`], and a repository adds
//! its own under `[guardrails] sections` (#77). A heading is matched by the
//! words it says, whatever their case and whatever else the heading says.

use crate::core::change::Change;
use crate::core::classify::{classify_file, FileKind};
use crate::core::finding::{Detection, Message, Region, Stamp};
use crate::core::guard;
use crate::core::rules::check::Judgement;

/// The files whose law lives in a section rather than in the whole file.
const INSTRUCTIONS: &[&str] = &["AGENTS.md", "CLAUDE.md"];

/// The words a heading says when the section under it states the law, in any
/// repository. A repository that writes its own adds them in `[guardrails]
/// sections`.
const LAW_HEADINGS: &[&str] = &["hard limits", "non-negotiable", "non-negotiables"];

/// The words that state the law only when they open the heading. "Never do
/// these" is a rule; "three signals, never one number" describes something
/// else, and the calibration read one such heading as a block on 2026-11-05.
const LAW_OPENERS: &[&str] = &["never"];

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let headings: Vec<String> = LAW_HEADINGS
        .iter()
        .map(ToString::to_string)
        .chain(judged.config.guardrail_sections.iter().cloned())
        .map(|heading| heading.trim().to_lowercase())
        .collect();
    judged
        .changes
        .iter()
        .filter_map(|change| edit(change, &headings))
        .collect()
}

fn edit(change: &Change, headings: &[String]) -> Option<Detection> {
    let path = change.path()?;
    if classify_file(path, "").kind != FileKind::Guardrail {
        return None;
    }
    if let Some(hook) = guard::hook_named(path) {
        if guard::is_own_bundle(hook, change.after.text()) {
            return None;
        }
    }
    if !is_instructions(path) {
        return Some(guardrail(path));
    }
    limits_line(change, headings).map(|line| limits(path, line))
}

/// Whether a path is one of the files weeder reads section by section.
fn is_instructions(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    INSTRUCTIONS.contains(&name)
}

/// The line the change first touched inside a section that states the law, on
/// either side of it. A limit taken out is gone from the new file, and a limit
/// added was never in the old one, so both sides are read.
fn limits_line(change: &Change, headings: &[String]) -> Option<u32> {
    let after = sections(change.after.text(), headings);
    let before = sections(change.before.text(), headings);
    let added = change
        .added()
        .filter(|(line, _)| after.iter().any(|range| range.holds(*line)))
        .map(|(line, _)| line)
        .min();
    let removed = change
        .removed()
        .filter(|(line, _)| before.iter().any(|range| range.holds(*line)))
        .map(|(line, _)| line)
        .min();
    match (added, removed) {
        (Some(line), _) => Some(line),
        // A hunk that only took lines out has no line of its own in the new
        // file; the finding points at where the first section still begins.
        (None, Some(_)) => after.first().map(|range| range.first).or(Some(1)),
        (None, None) => None,
    }
}

/// The lines a heading owns: itself, and everything under it until a heading of
/// its own rank or above.
#[derive(Debug, Clone, Copy)]
struct Section {
    first: u32,
    last: u32,
}

impl Section {
    fn holds(&self, line: u32) -> bool {
        self.first <= line && line <= self.last
    }
}

/// Every section a document states its law in, in the order it writes them.
fn sections(text: &str, headings: &[String]) -> Vec<Section> {
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let rank = heading_rank(line)?;
            let title = line.trim_start_matches('#').trim().to_lowercase();
            (headings.iter().any(|heading| says(&title, heading))
                || LAW_OPENERS.iter().any(|opener| opens(&title, opener)))
            .then_some((index, rank))
        })
        .map(|(index, rank)| {
            let last = lines
                .iter()
                .enumerate()
                .skip(index + 1)
                .find(|(_, line)| heading_rank(line).is_some_and(|next| next <= rank))
                .map_or(lines.len(), |(next, _)| next);
            Section {
                first: index as u32 + 1,
                last: last as u32,
            }
        })
        .collect()
}

/// Whether a heading's title says the words, as words: `never` is said by
/// "Never do these" and not by "Nevertheless".
fn says(title: &str, words: &str) -> bool {
    !words.is_empty()
        && title.match_indices(words).any(|(at, _)| {
            let before = title[..at].chars().next_back();
            let after = title[at + words.len()..].chars().next();
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
        })
}

/// Whether a heading's title begins with the word: "never do these" begins
/// with `never`, "never-ending" and "the cockpit, never one number" do not.
fn opens(title: &str, word: &str) -> bool {
    title.strip_prefix(word).is_some_and(|rest| {
        !rest
            .chars()
            .next()
            .is_some_and(|next| next.is_alphanumeric() || next == '-')
    })
}

/// How deep a markdown heading sits, or `None` for a line that is not one.
fn heading_rank(line: &str) -> Option<usize> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let rest = &line[hashes..];
    (hashes > 0 && (rest.is_empty() || rest.starts_with(' '))).then_some(hashes)
}

fn guardrail(path: &str) -> Detection {
    Detection {
        stamp: Stamp::Rule,
        path: path.to_string(),
        region: None,
        message: Message {
            what: "a guardrail file was changed.".to_string(),
            why: "this file decides what the checks do, so a change here can switch a gate off while every test still passes.".to_string(),
            next: "revert it, or land the guardrail change on its own with a `Weeder-allow: C1` trailer saying why, so a person reads it.".to_string(),
        },
        fix: None,
    }
}

fn limits(path: &str, line: u32) -> Detection {
    Detection {
        stamp: Stamp::Rule,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: "a section of the instructions that states their limits was changed.".to_string(),
            why: "these are the limits the work is held to, and an agent that edits them is widening what it is allowed to do.".to_string(),
            next: "revert the section, or land the change to the limits on its own with a `Weeder-allow: C1` trailer saying why, so a person reads it.".to_string(),
        },
        fix: None,
    }
}
