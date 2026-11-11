//! A release: the change that moves only a product's own version.
//!
//! A version bump touches files two rules guard for other reasons. A manifest
//! and its lockfile decide what the program is built from (D1); the garden's
//! `garden.json` declares which judges a planted repository runs (C1). A bump
//! moves neither of those things. It writes the new version where the old one
//! stood, in the manifest, in the lockfile's entry for the package itself, and
//! in the garden manifest's `version` and the asset URLs named after it. When
//! that is every line the change took out and put in, neither rule has
//! anything to say, and both ask this module rather than keeping a reading of
//! their own.
//!
//! Which `version` is the product's own is read from where it sits in the file,
//! because the same line of text pins a dependency one table further down, or a
//! judge one entry further down. A file that does not parse, or a format that
//! writes no own version, has no release lines, so every change to it is the
//! guarding rule's as before.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use serde::Deserialize;
use toml::Spanned;

use crate::core::change::Change;

/// Whether a path is a garden manifest, the one guardrail a release moves.
pub fn is_garden_manifest(path: &str) -> bool {
    Format::of(path) == Some(Format::GardenJson)
}

/// Whether every line the change took out and put in is one of `released`,
/// the lines [`releases`] found: the change is a release and nothing else.
pub fn only_releases(change: &Change, released: &BTreeSet<u32>) -> bool {
    !released.is_empty()
        && released.len() == change.added().count()
        && released.len() == change.removed().count()
}

/// The lines of the new file where the change moved the product's own version
/// and nothing else: a rewritten line whose old text writes the own version on
/// one side, whose new text writes it on the other, and whose every other
/// character stayed where it was. A file that moved, or one whose format
/// writes no own version, has none.
pub fn releases(change: &Change) -> BTreeSet<u32> {
    let format = match (&change.diff.old_path, &change.diff.new_path) {
        (Some(old), Some(new)) if old == new => Format::of(new),
        _ => None,
    };
    let Some(format) = format else {
        return BTreeSet::new();
    };
    let before = own_version_lines(
        change.before.text(),
        format.own_versions(change.before.text()),
    );
    let after = own_version_lines(
        change.after.text(),
        format.own_versions(change.after.text()),
    );
    change
        .replacements()
        .into_iter()
        .filter(|pair| {
            before
                .get(&pair.old_line)
                .is_some_and(|held| after.get(&pair.new_line) == Some(held))
        })
        .map(|pair| pair.new_line)
        .collect()
}

/// The files that write down the version a product calls itself. Every other
/// kind writes none: `go.mod` names a module and what it requires, and a Go
/// release is a tag, so any change to it moves what the program is built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    CargoToml,
    CargoLock,
    PackageJson,
    PackageLock,
    Pyproject,
    GardenJson,
}

impl Format {
    fn of(path: &str) -> Option<Format> {
        match path.rsplit('/').next().unwrap_or(path) {
            "Cargo.toml" => Some(Format::CargoToml),
            "Cargo.lock" => Some(Format::CargoLock),
            "package.json" => Some(Format::PackageJson),
            "package-lock.json" => Some(Format::PackageLock),
            "pyproject.toml" => Some(Format::Pyproject),
            "garden.json" => Some(Format::GardenJson),
            _ => None,
        }
    }

    /// Where the file writes the product's own version: the bytes of each
    /// value, in the places the format keeps it. A file that does not parse
    /// writes none, so every change to it is reported as before.
    fn own_versions(self, text: &str) -> Vec<Range<usize>> {
        match self {
            Format::CargoToml => {
                let Ok(manifest) = toml::from_str::<CargoManifest>(text) else {
                    return Vec::new();
                };
                let workspace = manifest.workspace.and_then(|workspace| workspace.package);
                [manifest.package, workspace]
                    .into_iter()
                    .flatten()
                    .filter_map(Versioned::span)
                    .collect()
            }
            // Cargo writes a `source` for every package it fetched, from a
            // registry or from git. An entry without one is a package of this
            // workspace, and its version is the release.
            Format::CargoLock => {
                let Ok(lock) = toml::from_str::<CargoLock>(text) else {
                    return Vec::new();
                };
                lock.package
                    .into_iter()
                    .filter(|entry| entry.source.is_none())
                    .filter_map(|entry| string_span(entry.version))
                    .collect()
            }
            Format::Pyproject => {
                let Ok(project) = toml::from_str::<Pyproject>(text) else {
                    return Vec::new();
                };
                let poetry = project.tool.and_then(|tool| tool.poetry);
                [project.project, poetry]
                    .into_iter()
                    .flatten()
                    .filter_map(Versioned::span)
                    .collect()
            }
            Format::PackageJson => json_values(text, &[&["version"]]),
            // npm writes the root package twice: at the top of the lockfile, and
            // again as the entry whose key is the empty path.
            Format::PackageLock => json_values(text, &[&["version"], &["packages", "", "version"]]),
            Format::GardenJson => garden_versions(text),
        }
    }
}

#[derive(Deserialize)]
struct CargoManifest {
    package: Option<Versioned>,
    workspace: Option<CargoWorkspace>,
}

#[derive(Deserialize)]
struct CargoWorkspace {
    package: Option<Versioned>,
}

#[derive(Deserialize)]
struct CargoLock {
    #[serde(default)]
    package: Vec<LockEntry>,
}

#[derive(Deserialize)]
struct LockEntry {
    version: Option<Spanned<toml::Value>>,
    source: Option<toml::Value>,
}

#[derive(Deserialize)]
struct Pyproject {
    project: Option<Versioned>,
    tool: Option<PythonTools>,
}

#[derive(Deserialize)]
struct PythonTools {
    poetry: Option<Versioned>,
}

/// A table that may carry a package's own `version`.
#[derive(Deserialize)]
struct Versioned {
    version: Option<Spanned<toml::Value>>,
}

impl Versioned {
    fn span(self) -> Option<Range<usize>> {
        string_span(self.version)
    }
}

/// Where a version written as a string sits. One written any other way, such
/// as `version.workspace = true`, names no release of its own here.
fn string_span(version: Option<Spanned<toml::Value>>) -> Option<Range<usize>> {
    version
        .filter(|value| value.get_ref().is_str())
        .map(|value| value.span())
}

/// Where the garden manifest writes the product's own version. The schema
/// keeps it in one place, the top-level `version`, "of the bed itself"; a
/// `version` anywhere deeper belongs to something else, such as a judge the
/// manifest pins, and moving that moves the checks. The release also writes
/// the version into each asset URL under `install.binaries`, as the
/// `v<version>` segment of its path, so those bytes are the release's too.
/// They are counted only where the segment names the version the same file
/// states, and only where that version is written the way the schema's
/// pattern writes one, so a version cannot walk a URL anywhere else.
fn garden_versions(text: &str) -> Vec<Range<usize>> {
    let Some(own) = json_values(text, &[&["version"]]).into_iter().next() else {
        return Vec::new();
    };
    let Some(version) = text
        .get(own.clone())
        .and_then(|written| written.strip_prefix('"'))
        .and_then(|written| written.strip_suffix('"'))
        .filter(|version| is_semver(version))
    else {
        return Vec::new();
    };
    let segment = format!("/v{version}");
    // A `binaries` that is not an object names no asset, so no URL line is the
    // release's and every one of them stays the guarding rule's.
    let urls = json_values(text, &[&["install", "binaries"]])
        .into_iter()
        .filter_map(|binaries| {
            Json {
                bytes: text.as_bytes(),
                at: binaries.start,
            }
            .members()
        })
        .flatten();
    let mut spans = vec![own.clone()];
    for url in urls {
        let Some(written) = text.get(url.clone()) else {
            continue;
        };
        for (at, _) in written.match_indices(&segment) {
            let end = at + segment.len();
            if matches!(written.as_bytes().get(end), Some(b'/' | b'"')) {
                let start = url.start + at + 2;
                spans.push(start..start + version.len());
            }
        }
    }
    spans
}

/// Whether a version is written the way semantic versioning writes one, which
/// is the pattern the garden schema holds `version` to: three numbers, and an
/// optional pre-release of letters, digits, dots and hyphens.
fn is_semver(version: &str) -> bool {
    let (core, pre) = match version.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (version, None),
    };
    let numbers: Vec<&str> = core.split('.').collect();
    numbers.len() == 3
        && numbers
            .iter()
            .all(|number| !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
        && pre.is_none_or(|pre| {
            !pre.is_empty()
                && pre
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
        })
}

/// Each line that writes the product's own version, by its number, as the
/// pieces of text around the versions written on it. Two sides of a rewritten
/// line with equal pieces differ only in the versions. A version whose bytes
/// run over a line break, or overlap another, leaves its line out.
fn own_version_lines(text: &str, mut spans: Vec<Range<usize>>) -> BTreeMap<u32, Vec<&str>> {
    spans.sort_by_key(|span| (span.start, span.end));
    spans.dedup();
    let mut lines: BTreeMap<u32, (Range<usize>, Vec<Range<usize>>)> = BTreeMap::new();
    let mut broken: BTreeSet<u32> = BTreeSet::new();
    for span in spans {
        let (Some(head), Some(value)) = (text.get(..span.start), text.get(span.clone())) else {
            continue;
        };
        let Ok(number) = u32::try_from(head.matches('\n').count() + 1) else {
            continue;
        };
        if value.contains('\n') {
            broken.insert(number);
            continue;
        }
        let start = head.rfind('\n').map_or(0, |at| at + 1);
        let end = text
            .get(span.end..)
            .and_then(|tail| tail.find('\n'))
            .map_or(text.len(), |at| span.end + at);
        let (_, held) = lines
            .entry(number)
            .or_insert_with(|| (start..end, Vec::new()));
        if held.last().is_some_and(|last| last.end > span.start) {
            broken.insert(number);
        }
        held.push(span);
    }
    lines
        .into_iter()
        .filter(|(number, _)| !broken.contains(number))
        .filter_map(|(number, (line, versions))| {
            let mut pieces = Vec::with_capacity(versions.len() + 1);
            let mut from = line.start;
            for version in versions {
                pieces.push(text.get(from..version.start)?);
                from = version.end;
            }
            pieces.push(text.get(from..line.end)?);
            Some((number, pieces))
        })
        .collect()
}

/// The bytes of the value at each path through a JSON document, for every path
/// the document has a value at. A document that is not JSON has none.
fn json_values(text: &str, paths: &[&[&str]]) -> Vec<Range<usize>> {
    if serde_json::from_str::<serde::de::IgnoredAny>(text).is_err() {
        return Vec::new();
    }
    paths
        .iter()
        .filter_map(|path| {
            Json {
                bytes: text.as_bytes(),
                at: 0,
            }
            .find(path)
        })
        .collect()
}

/// A walk through a JSON document that serde_json has already accepted, far
/// enough to say where one value sits. serde_json reads values and forgets
/// where they were; this reads positions and keeps no values.
struct Json<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Json<'_> {
    /// Where the value at the end of `path` sits, if there is one. Where a key
    /// is written twice the first is read.
    fn find(&mut self, path: &[&str]) -> Option<Range<usize>> {
        let Some((key, rest)) = path.split_first() else {
            return self.value();
        };
        self.eat(b'{')?;
        if self.peek()? == b'}' {
            return None;
        }
        loop {
            let name = self.string()?;
            self.eat(b':')?;
            if self.bytes.get(name) == Some(key.as_bytes()) {
                return self.find(rest);
            }
            self.value()?;
            self.eat(b',')?;
        }
    }

    /// Where each value of the object here sits, in the order it writes them.
    /// Anything here but an object has none.
    fn members(&mut self) -> Option<Vec<Range<usize>>> {
        self.eat(b'{')?;
        let mut values = Vec::new();
        if self.peek()? == b'}' {
            return Some(values);
        }
        loop {
            self.string()?;
            self.eat(b':')?;
            values.push(self.value()?);
            if self.peek()? == b'}' {
                return Some(values);
            }
            self.eat(b',')?;
        }
    }

    /// Steps over one value of any kind and says where it was.
    fn value(&mut self) -> Option<Range<usize>> {
        let opening = self.peek()?;
        let start = self.at;
        match opening {
            b'"' => {
                self.string()?;
            }
            b'{' | b'[' => {
                let mut depth = 0usize;
                loop {
                    match *self.bytes.get(self.at)? {
                        b'"' => {
                            self.string()?;
                            continue;
                        }
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth = depth.checked_sub(1)?;
                            if depth == 0 {
                                self.at += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    self.at += 1;
                }
            }
            _ => {
                while self.bytes.get(self.at).is_some_and(|byte| {
                    !matches!(byte, b',' | b'}' | b']') && !byte.is_ascii_whitespace()
                }) {
                    self.at += 1;
                }
            }
        }
        Some(start..self.at)
    }

    /// A string, and the bytes between its quotes.
    fn string(&mut self) -> Option<Range<usize>> {
        self.eat(b'"')?;
        let start = self.at;
        loop {
            match *self.bytes.get(self.at)? {
                b'"' => {
                    self.at += 1;
                    return Some(start..self.at - 1);
                }
                b'\\' => self.at = self.at.saturating_add(2),
                _ => self.at += 1,
            }
        }
    }

    fn eat(&mut self, byte: u8) -> Option<()> {
        (self.peek()? == byte).then(|| self.at += 1)
    }

    fn peek(&mut self) -> Option<u8> {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
        self.bytes.get(self.at).copied()
    }
}
