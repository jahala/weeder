//! `[scope] exclude`, the paths a repository states weeder does not judge.
//!
//! A repository keeps files that look dishonest on purpose: adversarial
//! fixtures, recorded runs of other repositories, a frozen copy of an old tree.
//! weeder reading them as production code is weeder being right about the wrong
//! file, so a repository names them, as patterns read from its root the way a
//! `.gitignore` line is ([`crate::core::pattern`]).
//!
//! Three things keep an exclusion from being a way round the gate. The law is
//! read from the base, so a pattern a change adds does not apply to that
//! change. `weeder.toml` is a guardrail file, so C1 blocks the change that
//! writes one until a person allows it. And a file moved into an excluded path
//! is judged as the deletion of the path it left.
//!
//! A guardrail file is never set aside, whatever pattern covers it. The parser
//! refuses a pattern that names one, and a pattern that only reaches one by a
//! wildcard, `**/testdata/` against `.githooks/testdata/`, leaves it judged.
//!
//! Nothing here hides. Each pattern that set a file aside comes back as one
//! note with a count, so a reader of the log sees the exclusion as plainly as
//! an allowance, and the log does not grow by a note per file.

use crate::core::classify::{classify_file, FileKind};
use crate::core::config::Config;
use crate::core::finding::{Finding, Level, Message};
use crate::core::glob;
use crate::core::pattern;

/// The id the note carries. It is not a rule: no detector reports it, `[rules]`
/// cannot turn it off, and a repository that stops excluding a path stops
/// hearing about it by taking the pattern out.
pub const NOTICE: &str = "EXCLUDED";

/// The first pattern that sets a path aside, or `None` where weeder judges it.
#[must_use]
pub fn covering<'a>(config: &'a Config, path: &str) -> Option<&'a str> {
    if guardrail(config, path) {
        return None;
    }
    config
        .exclude
        .iter()
        .map(String::as_str)
        .find(|candidate| pattern::covers(candidate, path))
}

/// Whether a pattern sets a path aside.
#[must_use]
pub fn excluded(config: &Config, path: &str) -> bool {
    covering(config, path).is_some()
}

/// Whether a path holds weeder's law or runs its checks: a file C1 guards, a
/// workflow, or a path `[guardrails] paths` holds at the constitution tier.
fn guardrail(config: &Config, path: &str) -> bool {
    matches!(
        classify_file(path, "").kind,
        FileKind::Guardrail | FileKind::Workflow
    ) || glob::matches_any(&config.guardrail_paths, path)
}

/// One note per pattern that set a file aside, counting the files it took. A
/// file two patterns cover is counted under the first, as [`covering`] reads
/// them, so the counts add up to the files set aside. A pattern that took
/// nothing this run says nothing.
#[must_use]
pub fn notices(config: &Config, set_aside: &[String]) -> Vec<Finding> {
    config
        .exclude
        .iter()
        .filter_map(|written| {
            let count = set_aside
                .iter()
                .filter(|path| covering(config, path) == Some(written.as_str()))
                .count();
            (count > 0).then(|| notice(written, count))
        })
        .collect()
}

fn notice(written: &str, count: usize) -> Finding {
    let files = if count == 1 { "file" } else { "files" };
    Finding {
        rule: NOTICE.to_string(),
        level: Level::Note,
        path: written.to_string(),
        region: None,
        message: Message {
            what: format!("weeder did not judge {count} {files} under {written}."),
            why: "[scope] exclude names the pattern, so no rule read what it covers.".to_string(),
            next: "read the exclusion as you would an allowance; take the pattern out of [scope] exclude to have weeder judge those files again.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
