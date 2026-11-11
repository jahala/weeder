//! The changed files weeder could not judge, said once per run.
//!
//! weeder's rules read TypeScript, JavaScript, Python, Rust, Go and bash. A
//! change in any other language reaches none of them, and a run over a change made only
//! of such files used to end with nothing to say, which a reader takes for a
//! clean bill. One note names what went unread, by extension and count, so a
//! clean result never passes for a judged one (#79).

use std::collections::BTreeMap;

use crate::core::finding::{Finding, Level, Message};

/// The id the note carries. It is not a rule: no detector reports it and
/// `[rules]` cannot turn it off, because the gap it names is weeder's own.
pub const NOTICE: &str = "UNREAD";

/// Where the note points: the repository itself, since it is about the run
/// rather than about any one file.
const ROOT: &str = ".";

/// The note for a run whose changed files include these, the paths weeder
/// knows for code in a language it carries no rules for. `None` where there are
/// none, so a run that read everything it was handed says nothing extra.
#[must_use]
pub fn notice(paths: &[&str]) -> Option<Finding> {
    if paths.is_empty() {
        return None;
    }
    // Keyed by the extension's spelling, so the order is the same on every run.
    let mut counted: BTreeMap<String, usize> = BTreeMap::new();
    for path in paths {
        *counted.entry(kind_of(path)).or_default() += 1;
    }
    let listed: Vec<String> = counted
        .iter()
        .map(|(kind, count)| format!("{count} {kind}"))
        .collect();
    let files = if paths.len() == 1 { "file" } else { "files" };
    Some(Finding {
        rule: NOTICE.to_string(),
        level: Level::Note,
        path: ROOT.to_string(),
        region: None,
        message: Message {
            what: format!(
                "weeder did not read {} changed {files}, in languages it has no rules for: {}.",
                paths.len(),
                listed.join(", ")
            ),
            why: "no rule judged a line of them, so a clean result says nothing about what they changed.".to_string(),
            next: "read those changes yourself; weeder judges TypeScript, JavaScript, Python, Rust, Go and bash.".to_string(),
        },
        fix: None,
        suppressed: None,
    })
}

/// How a path is counted: by its extension with the dot, `.sh`, or by its whole
/// name where it has none, `Makefile`.
fn kind_of(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() && !extension.is_empty() => {
            format!(".{}", extension.to_ascii_lowercase())
        }
        _ => name.to_string(),
    }
}
