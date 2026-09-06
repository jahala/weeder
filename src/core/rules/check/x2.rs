//! X2, a file outside the scope was touched.
//!
//! A scope is the sentence "this change is about that", written as globs, and a
//! file the change reached that the sentence does not cover is work nobody asked
//! for. It is how a small change grows: the fix needed one more thing, and that
//! thing needed one more thing.
//!
//! The finding carries the blast radius with it. What makes an out-of-scope edit
//! expensive is not the file, it is everything that was relying on the
//! definitions in it, so the callers the face found are named here, up to a few,
//! with a count of the rest.
//!
//! A run that was given no scope at all reports nothing. weed will not invent
//! the sentence a change was supposed to be held to.

use crate::core::change::Change;
use crate::core::finding::{Finding, Level, Message};
use crate::core::glob;
use crate::core::read::CallerSite;
use crate::core::rules::check::Judgement;

/// How many call sites one finding names before it starts counting instead. A
/// reviewer needs to know where to look, not to read a list.
const NAMED_CALLERS: usize = 3;

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let mut findings = Vec::new();
    for change in judged.changes {
        let Some(path) = change.path() else {
            continue;
        };
        if glob::matches_any(judged.scope, path) {
            continue;
        }
        findings.push(finding(path, &blast_radius(change, path, judged.callers)));
    }
    findings
}

/// What was relying on the definitions this change touched, said in one clause.
fn blast_radius(change: &Change, path: &str, callers: &[CallerSite]) -> String {
    let changed = change.changed_definitions();
    let sites: Vec<&CallerSite> = callers
        .iter()
        .filter(|site| changed.contains(&site.symbol))
        .filter(|site| site.path.to_string_lossy() != path)
        .collect();
    if sites.is_empty() {
        return "nothing else in the tree calls what it changed".to_string();
    }
    let named: Vec<String> = sites
        .iter()
        .take(NAMED_CALLERS)
        .map(|site| format!("{} calls {}", site.path.display(), site.symbol))
        .collect();
    match sites.len().saturating_sub(named.len()) {
        0 => named.join(", "),
        rest => format!("{}, and {rest} more", named.join(", ")),
    }
}

fn finding(path: &str, radius: &str) -> Finding {
    Finding {
        rule: "X2".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: None,
        message: Message {
            what: "a file outside the scope this run was given was changed.".to_string(),
            why: format!("nobody asked for this edit, and {radius}."),
            next: "revert the file, or widen the scope on purpose so the change is the one that was asked for.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
