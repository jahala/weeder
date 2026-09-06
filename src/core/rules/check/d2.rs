//! D2, an import crossed a boundary the repository forbids.
//!
//! `[deps] layers` names the parts a repository is built from as path globs, and
//! `[deps] allow` names the directions an import may run between them. Anything
//! else is an arrow drawn the wrong way: one line to write, and a long time to
//! take back out once everything below it depends on everything above.
//!
//! An import names a module, and weed has to work out which file that module is.
//! It does that against the repository's own paths rather than against a build
//! system: the module's name is broken into segments, a relative one is resolved
//! from the file that wrote it, and the repository path whose own segments end
//! in the longest run of those is the file the import points at. Languages spell
//! this differently, a dotted package, a crate path, a module path with a domain
//! in front of it, and all of them come apart into the same segments.
//!
//! A changed file is read whole, imports and all. An arrow that was already
//! there is one this change is now standing on.

use std::collections::BTreeMap;

use crate::core::config::DependencyDirection;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::glob;
use crate::core::read::Import;
use crate::core::rules::check::Judgement;

/// The extensions a module name may carry that are not part of the module.
const CODE_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "rs", "go"];

/// What separates one segment of a module name from the next, in every language
/// weed reads, together with the punctuation a statement wraps them in.
const SEPARATORS: [char; 9] = ['/', ':', '.', '{', '}', ',', ' ', '"', '\''];

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let layers = &judged.config.layers;
    if layers.is_empty() {
        return Vec::new();
    }
    let mut findings = Vec::new();
    for change in judged.changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        let Some(from) = layer_of(path, layers) else {
            continue;
        };
        for import in &change.after.imports {
            let Some(to) = points_at(path, &import.source, judged) else {
                continue;
            };
            if to == from || allows(&judged.config.dependency_directions, &from, &to) {
                continue;
            }
            findings.push(finding(path, import, &from, &to));
        }
    }
    findings
}

/// The layer a path belongs to, or `None` for a path no layer claims. Where two
/// layers claim one path, the first by name wins, so the answer never depends on
/// the order a config file happened to list them in.
fn layer_of(path: &str, layers: &BTreeMap<String, Vec<String>>) -> Option<String> {
    layers
        .iter()
        .find(|(_, globs)| glob::matches_any(globs, path))
        .map(|(layer, _)| layer.clone())
}

/// Whether the repository allows an import to run from one layer to another.
fn allows(directions: &[DependencyDirection], from: &str, to: &str) -> bool {
    directions
        .iter()
        .any(|direction| direction.from == from && direction.to == to)
}

/// The layer an import points at, or `None` where it points outside the
/// repository or at more than one layer at once. A rule that blocks does not
/// report a guess.
fn points_at(importer: &str, source: &str, judged: &Judgement) -> Option<String> {
    let named = segments(importer, source);
    if named.is_empty() {
        return None;
    }
    let mut best = 0;
    let mut found: Option<String> = None;
    for path in judged.paths {
        let score = resolves(&named, path);
        if score == 0 || score < best {
            continue;
        }
        let layer = layer_of(path, &judged.config.layers);
        if score > best {
            best = score;
            found = layer;
            continue;
        }
        // Two paths answer the import equally well. They are the same answer
        // only where they sit in the same layer; otherwise weed does not know.
        if layer != found {
            found = None;
        }
    }
    found
}

/// How well a repository path answers a module name: the longest run of the
/// path's own last segments that the name spells out. The path is read both
/// whole and as the directory it sits in, because a language that imports a
/// package imports the directory rather than a file in it.
fn resolves(named: &[String], path: &str) -> usize {
    let whole = path_segments(path);
    let directory = whole.split_last().map(|(_, rest)| rest).unwrap_or_default();
    tail_within(&whole, named).max(tail_within(directory, named))
}

/// The longest run of `candidate`'s last segments that appears whole inside
/// `named`, or 0 where none of it does.
fn tail_within(candidate: &[String], named: &[String]) -> usize {
    (1..=candidate.len())
        .rev()
        .find(|length| {
            let tail = &candidate[candidate.len() - length..];
            named.windows(*length).any(|window| window == tail)
        })
        .unwrap_or(0)
}

/// A repository path as its segments, with the file's extension left off: an
/// import names a module, and the extension is the filesystem's business.
fn path_segments(path: &str) -> Vec<String> {
    let mut segments: Vec<String> = path.split('/').map(ToString::to_string).collect();
    if let Some(last) = segments.last_mut() {
        if let Some((stem, extension)) = last.rsplit_once('.') {
            if !stem.is_empty() && is_extension(extension) {
                *last = stem.to_string();
            }
        }
    }
    segments
}

/// A module name as its segments, resolved from the file that wrote it where it
/// was written relative to that file. One leading dot is the file's own
/// directory and each one after it is a step up, which is how both a path-style
/// import and a dotted package spell the same thing.
fn segments(importer: &str, source: &str) -> Vec<String> {
    let source = source.trim();
    let Some((climb, named)) = relative(source) else {
        return split(source);
    };
    let mut here: Vec<String> = path_segments(importer);
    // The file's own name first, then one directory for every step up.
    here.pop();
    for _ in 0..climb {
        here.pop();
    }
    here.extend(split(&named));
    here
}

/// How far a relative name climbs before it names anything, and what is left of
/// it once it has. The two spellings count differently and mean the same thing:
/// a path writes each step up as `..`, and a dotted package writes the first
/// dot for the directory it is in and one more dot for every step above it. A
/// name that is not relative climbs nothing and is answered whole.
fn relative(source: &str) -> Option<(usize, String)> {
    if !source.starts_with('.') {
        return None;
    }
    if !source.contains('/') {
        let dots = source
            .chars()
            .take_while(|character| *character == '.')
            .count();
        return Some((dots.saturating_sub(1), source[dots..].to_string()));
    }
    let mut climb = 0;
    let mut rest = source;
    loop {
        if let Some(tail) = rest.strip_prefix("../") {
            climb += 1;
            rest = tail;
            continue;
        }
        if let Some(tail) = rest.strip_prefix("./") {
            rest = tail;
            continue;
        }
        break;
    }
    if rest == ".." {
        climb += 1;
        rest = "";
    }
    Some((climb, rest.to_string()))
}

/// A module name broken into the words it is written from, with an extension
/// left off the end of it.
fn split(source: &str) -> Vec<String> {
    let mut segments: Vec<String> = source
        .split(SEPARATORS)
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect();
    if segments.len() > 1 && segments.last().is_some_and(|last| is_extension(last)) {
        segments.pop();
    }
    segments
}

fn is_extension(word: &str) -> bool {
    CODE_EXTENSIONS.contains(&word)
}

fn finding(path: &str, import: &Import, from: &str, to: &str) -> Finding {
    Finding {
        rule: "D2".to_string(),
        level: Level::Block,
        path: path.to_string(),
        region: Some(Region {
            start_line: import.start_line,
            end_line: import.end_line,
        }),
        message: Message {
            what: format!("`{from}` imports `{to}`, which this repository's [deps] does not allow."),
            why: "the layers only hold while every arrow between them runs the way the repository says it does.".to_string(),
            next: format!("move what `{from}` needs to where it may reach it, or state the direction in [deps] allow and say why."),
        },
        fix: None,
        suppressed: None,
    }
}
