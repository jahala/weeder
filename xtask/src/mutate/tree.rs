//! The repository as one case sees it: the commit checked out, the files it
//! holds, and what the commit itself changed.
//!
//! A commit's own diff matters to three of the rules. T5 is about an expectation
//! that moved alongside production code, M1 about a double of something the
//! change touched, X2 about a file the change was never scoped for, and all
//! three are only a shape at all because the commit already touched something.
//!
//! Three kinds of file are never planted in, and the report says how many were
//! passed over for each. A file the commit itself added has no earlier version,
//! so a shape about something being taken away could not be planted in it
//! honestly. A file carrying a NUL byte is one git writes no text diff for and
//! weed judges no lines in, by the same test git uses and for the reason weed
//! states: those bytes carry no lines. And a file that announces it was
//! generated is nobody's hand-written code, so a stub or a print left in it is
//! a fact about a generator rather than about a change.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::source::{Language, Source};

/// One commit of one repository, checked out and read.
pub struct Tree {
    pub root: PathBuf,
    /// Every path the commit's tree holds, sorted.
    pub paths: Vec<String>,
    /// The paths the commit itself changed against its parent, sorted.
    pub changed: Vec<String>,
    /// How many files were passed over as sites, by the reason.
    pub passed_over: BTreeMap<Passed, usize>,
    /// Every source file of every language this repository is read for, scanned
    /// once for the whole commit.
    sources: BTreeMap<Language, Vec<Source>>,
}

/// Why a file is no site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Passed {
    /// The commit brought the file into the repository.
    Added,
    /// The bytes carry no lines.
    NotText,
    /// The file says a generator wrote it.
    Generated,
}

impl Passed {
    pub fn spelled(self) -> &'static str {
        match self {
            Passed::Added => "added by the commit, so nothing in it was there to change",
            Passed::NotText => "carrying a NUL byte, which git writes no text diff for",
            Passed::Generated => "saying a generator wrote it",
        }
    }
}

impl Tree {
    pub fn read(
        root: &Path,
        paths: Vec<String>,
        changed: Vec<String>,
        added: &[String],
        languages: &[Language],
    ) -> Tree {
        let mut sources = BTreeMap::new();
        let mut passed_over = BTreeMap::new();
        for lang in languages {
            let mut scanned = Vec::new();
            for path in paths.iter().filter(|path| lang.owns(path)) {
                let Some(text) = text_at(root, path) else {
                    continue;
                };
                // A file of three lines is a re-export or somebody's stub, and
                // there is nothing in it to plant.
                if text.lines().count() < 4 {
                    continue;
                }
                if let Some(reason) = passed(path, &text, added) {
                    *passed_over.entry(reason).or_default() += 1;
                    continue;
                }
                scanned.push(Source::read(path, *lang, &text));
            }
            sources.insert(*lang, scanned);
        }
        Tree {
            root: root.to_path_buf(),
            paths,
            changed,
            passed_over,
            sources,
        }
    }

    /// A file's text, or nothing where it is gone, unreadable, or holds bytes
    /// that are not text.
    pub fn text(&self, path: &str) -> Option<String> {
        text_at(&self.root, path).filter(|text| !text.contains('\0'))
    }

    /// Every file of a language the commit holds.
    pub fn sources(&self, lang: Language) -> &[Source] {
        self.sources.get(&lang).map_or(&[], Vec::as_slice)
    }

    /// The files of a language that hold tests.
    pub fn tests(&self, lang: Language) -> Vec<&Source> {
        self.sources(lang)
            .iter()
            .filter(|source| source.holds_tests())
            .collect()
    }

    /// The files of a language no runner collects: production code.
    pub fn production(&self, lang: Language) -> Vec<&Source> {
        self.sources(lang)
            .iter()
            .filter(|source| !source.holds_tests() && !lang.collects_file(&source.path))
            .collect()
    }

    /// Whether the commit's own change touched this path.
    pub fn touched(&self, path: &str) -> bool {
        self.changed.iter().any(|changed| changed == path)
    }
}

/// Why this file is no site, where it is none.
fn passed(path: &str, text: &str, added: &[String]) -> Option<Passed> {
    if added.iter().any(|held| held == path) {
        return Some(Passed::Added);
    }
    if text.contains('\0') {
        return Some(Passed::NotText);
    }
    is_generated(text).then_some(Passed::Generated)
}

/// Whether a file says a generator wrote it, in the words every generator uses:
/// Go standardised one of these, and the rest are the same sentence.
fn is_generated(text: &str) -> bool {
    text.lines().take(5).any(|line| {
        let spelled = line.to_ascii_lowercase();
        spelled.contains("generated by")
            || spelled.contains("auto-generated")
            || spelled.contains("autogenerated")
            || spelled.contains("@generated")
            || spelled.contains("do not edit")
            || spelled.contains("do not hand-edit")
    })
}

fn text_at(root: &Path, path: &str) -> Option<String> {
    std::fs::read(root.join(path))
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|text| !text.is_empty())
}
