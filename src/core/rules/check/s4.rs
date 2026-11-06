//! S4, production code was added that only its tests call.
//!
//! An agent writes a function and a test for it, and never wires a caller. The
//! suite passes and the change reads as finished, and nothing the product runs
//! reaches the new code. R2 cannot see it, because the test is a reference.
//!
//! So S4 judges what the change exported: a definition a production file gains,
//! or one it held and now exports. Every reference to that name outside the
//! file that declares it has to be a test's, and at least one has to be there.
//! A reference from the same file does not wire it in, because the export is
//! what the change added and only the tests reach across for it.
//!
//! A reference is a call the face's caller search found anywhere in the tree, or
//! the name written in the code of a file the change touched. The search sees
//! calls alone: a function handed to `map` is used without being called, and a
//! call inside a Rust macro is one the parser never sees as a call. A new name
//! can only be written by a file the change wrote it into, so reading the
//! changed files covers both. A name some file exported before the change is a
//! move or an edit rather than new code, and is left alone.
//!
//! What something outside the repository calls by design is `rules::export`'s
//! answer, the one R2 reads, and a module `[entrypoints] cli` names is the
//! command line itself. Neither is judged.

use std::collections::{BTreeMap, BTreeSet};

use crate::core::change::{Change, Side};
use crate::core::classify::{classify_file, FileKind, Lang};
use crate::core::finding::{Detection, Message, Region, Stamp};
use crate::core::glob;
use crate::core::read::{CallerSite, Definition};
use crate::core::rules::check::Judgement;
use crate::core::rules::export;
use crate::core::syntax::words;

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let added: Vec<Added> = added_exports(judged.changes)
        .into_iter()
        .filter(|added| !glob::matches_any(&judged.config.cli_entrypoints, added.path))
        .collect();
    if added.is_empty() {
        return Vec::new();
    }
    let wanted: BTreeSet<&str> = added
        .iter()
        .map(|added| added.definition.name.as_str())
        .collect();
    let mut references = written_in_changes(judged.changes, &wanted);
    for site in judged.callers {
        if wanted.contains(site.symbol.as_str()) {
            references
                .entry(site.symbol.clone())
                .or_default()
                .insert(Reference {
                    path: site.path.to_string_lossy().into_owned(),
                    from_a_test: from_a_test(site),
                });
        }
    }

    added
        .into_iter()
        .filter(|added| only_tests_reach(added, references.get(&added.definition.name)))
        .map(|added| finding(&added))
        .collect()
}

/// The names the caller search has to be asked about for S4 to judge this
/// change: what it exported. The face asks once, for every rule that needs it.
#[must_use]
pub fn symbols_in_question(changes: &[Change]) -> BTreeSet<String> {
    added_exports(changes)
        .into_iter()
        .map(|added| added.definition.name.clone())
        .collect()
}

/// An export the change added, and the production file it is declared in.
struct Added<'a> {
    path: &'a str,
    definition: &'a Definition,
}

/// A file that names an export, and whether it is a test.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Reference {
    path: String,
    from_a_test: bool,
}

/// Every export a production file gained in this change. A name some file
/// exported before the change is not new, wherever it now sits.
fn added_exports(changes: &[Change]) -> Vec<Added<'_>> {
    let held: BTreeSet<&str> = changes
        .iter()
        .flat_map(|change| exported(&change.before))
        .map(|definition| definition.name.as_str())
        .collect();
    let mut added = Vec::new();
    for change in changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !change.after.is(FileKind::Prod) || export::is_entry_file(file_name(path)) {
            continue;
        }
        for definition in exported(&change.after) {
            if export::is_exempt(definition) || held.contains(definition.name.as_str()) {
                continue;
            }
            added.push(Added { path, definition });
        }
    }
    added
}

/// The definitions one side of a change exports, at the top of the file.
fn exported(side: &Side) -> impl Iterator<Item = &Definition> {
    let lang = side.lang();
    let mask = side.mask();
    side.outline.definitions.iter().filter(move |definition| {
        export::is_public(lang, definition, &mask.code(definition.start_line))
    })
}

/// The changed files that write each wanted name in their code, outside a
/// comment or a string.
fn written_in_changes(
    changes: &[Change],
    wanted: &BTreeSet<&str>,
) -> BTreeMap<String, BTreeSet<Reference>> {
    let mut written: BTreeMap<String, BTreeSet<Reference>> = BTreeMap::new();
    for change in changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if change.after.lang() == Lang::Other {
            continue;
        }
        let from_a_test = change.after.is(FileKind::Test);
        let mask = change.after.mask();
        for line in 1..=mask.line_count() {
            for word in words(&mask.code(line)) {
                if wanted.contains(word.text) {
                    written
                        .entry(word.text.to_string())
                        .or_default()
                        .insert(Reference {
                            path: path.to_string(),
                            from_a_test,
                        });
                }
            }
        }
    }
    written
}

/// Whether the caller search found this call in a test file.
fn from_a_test(site: &CallerSite) -> bool {
    classify_file(&site.path, "").kind == FileKind::Test
}

/// Whether every reference to the export outside its own file is a test's,
/// with at least one of them there.
fn only_tests_reach(added: &Added, references: Option<&BTreeSet<Reference>>) -> bool {
    let mut outside = references
        .into_iter()
        .flatten()
        .filter(|reference| reference.path != added.path)
        .peekable();
    outside.peek().is_some() && outside.all(|reference| reference.from_a_test)
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn finding(added: &Added) -> Detection {
    let name = &added.definition.name;
    Detection {
        stamp: Stamp::Rule,
        path: added.path.to_string(),
        region: Some(Region {
            start_line: added.definition.start_line,
            end_line: added.definition.end_line,
        }),
        message: Message {
            what: format!("`{name}` was exported by this change, and only tests reach it from outside its own file."),
            why: "code only its tests call is behaviour nothing in the product uses: the suite passes, and no path the program runs reaches it.".to_string(),
            next: format!("wire `{name}` into the path that needs it, or take it and its tests out; if it is staged for a later change on purpose, say so in a `Weeder-allow: S4` trailer."),
        },
        fix: None,
    }
}
