//! `[scope] specimens`, the paths a repository states weed does not judge.
//!
//! An adversarial fixture is written to look dishonest, and weed reading it as
//! though it were production code is weed being right about the wrong file. A
//! repository says which paths those are, and what it may say is bounded on
//! purpose: directories under [`ROOT`], never a rule, never a pattern that can
//! name a path somewhere else. An exclusion that could reach anywhere is a rule
//! turned off with none of the words that turning a rule off takes.
//!
//! Nothing here hides. Every path an exclusion covers comes back as a note, once
//! for the path rather than once per rule, so a reader of the log sees the
//! exclusion as plainly as they would see an allowance.

use crate::core::catalogue;
use crate::core::finding::{Finding, Level, Message};
use crate::core::glob;

/// The one root a specimen exclusion may name. Fixtures live in one place so
/// that an exclusion cannot be written for anything else.
pub const ROOT: &str = "fixtures/adversarial";

/// The id the note carries. It is not a rule: no detector reports it, `[rules]`
/// cannot turn it off, and a repository that stops excluding a path stops
/// hearing about it by taking the entry out.
pub const NOTICE: &str = "SPECIMEN";

/// Why an entry cannot be a specimen exclusion, or `None` where it can. The
/// message names the entry it refused: the caller has a list, and a refusal that
/// does not say which line of it was wrong sends the reader back to guess.
#[must_use]
pub fn refusal(entry: &str) -> Option<String> {
    if catalogue::rule(&entry.trim().to_ascii_uppercase()).is_some() {
        return Some(format!(
            "`{entry}` is a rule, and this is a list of directories. state a rule's level under [rules], where a repository can read what it turned off; an exclusion here would turn it off for the paths nobody looks at"
        ));
    }
    if !confined(entry) {
        return Some(if reaches(entry) {
            format!(
                "`{entry}` can name a path outside {ROOT}/, and a specimen exclusion may not reach past it. name a directory under {ROOT}/"
            )
        } else {
            format!(
                "`{entry}` is outside {ROOT}/, the one root a specimen exclusion may name. move the specimens under it, or let weed judge them"
            )
        });
    }
    None
}

/// Whether every path this entry can name is the root or lies under it.
///
/// The entry is read as the glob it is: `*` and `**` may stand anywhere past the
/// root, so a repository can exclude `fixtures/adversarial/**/after` without
/// listing every rule's directory. What it may not do is put a wildcard where
/// the root belongs, because `fixtures/*` names `fixtures/vendor` as readily as
/// it names the fixtures, and it may not walk back out with `..`.
fn confined(entry: &str) -> bool {
    let path = entry.trim_end_matches('/');
    if path.split('/').any(|segment| segment == "..") {
        return false;
    }
    path == ROOT || path.starts_with(&format!("{ROOT}/"))
}

/// Whether the entry gets outside the root by reaching rather than by pointing
/// somewhere else altogether: a wildcard wide enough to leave, or a step back up
/// the tree. It decides which of two refusals a reader is handed, and both of
/// them refuse.
fn reaches(entry: &str) -> bool {
    entry
        .split('/')
        .any(|segment| segment == ".." || segment.contains('*') || segment.contains('?'))
}

/// Whether a repository-relative path is one the exclusions cover. An entry
/// names a directory, so the paths inside it are covered too.
#[must_use]
pub fn skipped(specimens: &[String], path: &str) -> bool {
    specimens.iter().any(|entry| {
        let entry = entry.trim_end_matches('/');
        glob::matches(entry, path) || glob::matches(&format!("{entry}/**"), path)
    })
}

/// What weed says about a path it did not judge.
#[must_use]
pub fn notice(path: &str) -> Finding {
    Finding {
        rule: NOTICE.to_string(),
        level: Level::Note,
        path: path.to_string(),
        region: None,
        message: Message {
            what: format!("weed did not judge {path}."),
            why: "[scope] specimens names it as an adversarial fixture, which is written to look dishonest, so no rule read it.".to_string(),
            next: "read the exclusion as you would an allowance; take the entry out of [scope] specimens to have weed judge the path again.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
