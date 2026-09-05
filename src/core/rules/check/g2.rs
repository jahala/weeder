//! G2, a large or a binary file was added.
//!
//! A repository is text somebody can read a diff of. A blob that arrives once is
//! carried by every clone from then on, and the usual way one gets in is that a
//! build wrote it and everything was staged at once.
//!
//! Only an addition is reported. A file the repository already carried was said
//! yes to before this change, and saying it again every time somebody edits it
//! would teach a reviewer to look away.

use crate::core::change::Change;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::rules::check::Judgement;

/// The weight above which a file stops being something anyone reads.
const MEBIBYTE: u64 = 1024 * 1024;

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    judged.changes.iter().filter_map(added).collect()
}

fn added(change: &Change) -> Option<Finding> {
    if !change.is_addition() {
        return None;
    }
    let path = change.diff.new_path.as_deref()?;
    let weight = weight(change);
    if change.after.binary {
        return Some(finding(path, change, Blob::binary(&weight)));
    }
    (change.after.size? > MEBIBYTE).then(|| finding(path, change, Blob::large(&weight)))
}

/// What the added file weighs, written the way a person reads a size.
fn weight(change: &Change) -> String {
    let bytes = change.after.size.unwrap_or_default();
    if bytes >= MEBIBYTE {
        format!("{} MiB", bytes / MEBIBYTE)
    } else {
        format!("{bytes} bytes")
    }
}

/// What was added, as it will be put to the reader.
struct Blob {
    what: String,
    why: &'static str,
}

impl Blob {
    fn large(weight: &str) -> Blob {
        Blob {
            what: format!("a file of {weight} was added."),
            why: "every clone of this repository carries it from now on, and no diff of it is readable.",
        }
    }

    fn binary(weight: &str) -> Blob {
        Blob {
            what: format!("a file of {weight} was added whose bytes are not text."),
            why: "nobody can review what changed in it, and no later change to it can be read either.",
        }
    }
}

fn finding(path: &str, change: &Change, blob: Blob) -> Finding {
    Finding {
        rule: "G2".to_string(),
        level: Level::Warn,
        path: path.to_string(),
        region: change.first_edit().map(|line| Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: blob.what,
            why: blob.why.to_string(),
            next: "keep it out of the repository, or say where it came from and how it is rebuilt."
                .to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
