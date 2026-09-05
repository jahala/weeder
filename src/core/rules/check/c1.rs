//! C1 — a guardrail was edited.
//!
//! A guardrail is a file that decides what the other checks do: the workflow
//! that runs them, the settings an agent harness reads, the hook git calls,
//! weed's own law. A change there can turn every gate off without touching a
//! single test, which is why any change to one is the finding — added, edited or
//! taken away, and whatever it says.
//!
//! One file under `.githooks/` is not an edit: the bundle weed itself writes,
//! byte for byte, for the binary and the branches it names. Adopting weed is a
//! change the gate lets through, because weed can tell its own hook from one
//! somebody rewrote; a hook that differs by one byte is a guardrail edit again.
//!
//! The two markdown files are different. `AGENTS.md` and `CLAUDE.md` are mostly
//! prose that wants editing, and one section of them is law. So weed reads the
//! headings, finds the section that states the hard limits, and reports only a
//! hunk that lands inside it. A paragraph rewritten three sections down is
//! writing; a line changed under the limits is a rule being rewritten.

use crate::core::change::Change;
use crate::core::classify::{classify_file, FileKind};
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::guard;

/// The files whose law lives in one section rather than in the whole file.
const INSTRUCTIONS: &[&str] = &["AGENTS.md", "CLAUDE.md"];

/// The heading that opens the section those files state their law in.
const HARD_LIMITS: &str = "hard limits";

pub fn evaluate(changes: &[Change]) -> Vec<Finding> {
    changes.iter().filter_map(edit).collect()
}

fn edit(change: &Change) -> Option<Finding> {
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
    limits_line(change).map(|line| limits(path, line))
}

/// Whether a path is one of the files weed reads section by section.
fn is_instructions(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    INSTRUCTIONS.contains(&name)
}

/// The line the change first touched inside the hard-limits section, on either
/// side of it. A limit taken out is gone from the new file, and a limit added
/// was never in the old one, so both sides are read.
fn limits_line(change: &Change) -> Option<u32> {
    let after = section(change.after.text());
    let before = section(change.before.text());
    let added = change
        .added()
        .filter(|(line, _)| after.is_some_and(|range| range.holds(*line)))
        .map(|(line, _)| line)
        .min();
    let removed = change
        .removed()
        .filter(|(line, _)| before.is_some_and(|range| range.holds(*line)))
        .map(|(line, _)| line)
        .min();
    match (added, removed) {
        (Some(line), _) => Some(line),
        // A hunk that only took lines out has no line of its own in the new
        // file; the finding points at where the section still begins.
        (None, Some(_)) => after.map(|range| range.first).or(Some(1)),
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

/// Where a document states its hard limits, or `None` where it states none.
fn section(text: &str) -> Option<Section> {
    let lines: Vec<&str> = text.lines().collect();
    let (index, rank) = lines.iter().enumerate().find_map(|(index, line)| {
        let rank = heading_rank(line)?;
        let title = line.trim_start_matches('#').trim().to_ascii_lowercase();
        title.contains(HARD_LIMITS).then_some((index, rank))
    })?;
    let last = lines
        .iter()
        .enumerate()
        .skip(index + 1)
        .find(|(_, line)| heading_rank(line).is_some_and(|next| next <= rank))
        .map_or(lines.len(), |(next, _)| next);
    Some(Section {
        first: index as u32 + 1,
        last: last as u32,
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

fn guardrail(path: &str) -> Finding {
    Finding {
        rule: "C1".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: None,
        message: Message {
            what: "a guardrail file was changed.".to_string(),
            why: "this file decides what the checks do, so a change here can switch a gate off while every test still passes.".to_string(),
            next: "revert it, or land the guardrail change on its own so a person reads it.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

fn limits(path: &str, line: u32) -> Finding {
    Finding {
        rule: "C1".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: "the hard-limits section of the instructions was changed.".to_string(),
            why: "these are the limits the work is held to, and an agent that edits them is widening what it is allowed to do.".to_string(),
            next: "revert the section, or land the change to the limits on its own so a person reads it.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
