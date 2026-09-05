//! A changed file, and everything a rule reads about it.
//!
//! A detector is pure, so whatever it needs about a file has to arrive as data.
//! That is a [`Change`]: the hunks git wrote, and each side of the change as the
//! face gathered it — the text, what the path classifies as, the tests the file
//! declares and what it defines. The face reads those through the seams; core
//! only ever looks at them, which is what keeps a rule runnable on a diff that
//! never touched a disk.

use crate::core::classify::{Classification, FileKind, Lang};
use crate::core::diff::{ChangeKind, FileDiff, HunkLine, LineKind};
use crate::core::read::{Outline, TestShape};
use crate::core::syntax::Mask;

/// One side of a change: the file as a ref carries it, or as the tree does.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Side {
    /// The file's text. `None` where this side has no such file — an added file
    /// has no before, a deleted one has no after — and where what it carries is
    /// not text, because weed judges lines and bytes carry none.
    pub content: Option<String>,
    /// What the path is, or `None` where this side has no file at all.
    pub classification: Option<Classification>,
    /// The tests the file declares, empty where it declares none.
    pub tests: TestShape,
    /// What the file defines, empty where it defines nothing.
    pub outline: Outline,
}

impl Side {
    /// The file's text, or nothing at all where this side has no file.
    #[must_use]
    pub fn text(&self) -> &str {
        self.content.as_deref().unwrap_or_default()
    }

    /// Whether this side's file is of that kind. A side with no file is nothing.
    #[must_use]
    pub fn is(&self, kind: FileKind) -> bool {
        self.classification
            .as_ref()
            .is_some_and(|classification| classification.kind == kind)
    }

    /// Whether this side's file carries tests: a test file, or a production
    /// file with a test module written inside it.
    #[must_use]
    pub fn holds_tests(&self) -> bool {
        self.is(FileKind::Test)
            || self
                .classification
                .as_ref()
                .is_some_and(|classification| classification.has_inline_tests)
    }

    /// How many cases the file declares.
    #[must_use]
    pub fn case_count(&self) -> usize {
        self.tests.cases().count()
    }

    /// The file read as the language it is written in.
    #[must_use]
    pub fn mask(&self) -> Mask {
        Mask::of(self.lang(), self.text())
    }

    fn lang(&self) -> Lang {
        self.classification
            .as_ref()
            .map_or(Lang::Other, |classification| classification.lang)
    }
}

/// One file the change touched, both sides of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub diff: FileDiff,
    pub before: Side,
    pub after: Side,
}

impl Change {
    /// The path a finding names: where the file is now, or where it was when
    /// the change took it away.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.diff
            .new_path
            .as_deref()
            .or(self.diff.old_path.as_deref())
    }

    /// Whether the change took the file away.
    #[must_use]
    pub fn is_deletion(&self) -> bool {
        self.diff.change == ChangeKind::Deleted
    }

    /// The lines the change added, each with the number it takes in the new
    /// file. A line git could not number is not one a finding can point at.
    pub fn added(&self) -> impl Iterator<Item = (u32, &str)> {
        self.lines(LineKind::Added)
            .filter_map(|line| Some((line.0?, line.1)))
    }

    /// The lines the change took out, each with the number it had.
    pub fn removed(&self) -> impl Iterator<Item = (u32, &str)> {
        self.diff
            .hunks
            .iter()
            .flat_map(|hunk| hunk.lines.iter())
            .filter(|line| line.kind == LineKind::Removed)
            .filter_map(|line| Some((line.old_line?, line.text.as_str())))
    }

    /// Where in the new file the change first touched something, so a finding
    /// about the whole file points at the edit rather than at line one of a
    /// file nobody edited there.
    #[must_use]
    pub fn first_edit(&self) -> Option<u32> {
        let added = self.added().map(|(line, _)| line).min();
        let hunks = self
            .diff
            .hunks
            .iter()
            .map(|hunk| hunk.new_start.max(1))
            .min();
        added.or(hunks)
    }

    /// The lines the change rewrote: a removed line and the added line that
    /// took its place.
    ///
    /// git writes a rewrite as a run of removals followed by a run of
    /// additions, and pairs nothing itself. weed pairs them by their order
    /// inside the run, which is the order an editor made them in: the first
    /// line out is the one the first line in replaced. A run whose two sides
    /// are of different lengths pairs as far as the shorter one goes, and the
    /// rest are additions and removals with no partner.
    #[must_use]
    pub fn replacements(&self) -> Vec<Replacement<'_>> {
        let mut pairs = Vec::new();
        for hunk in &self.diff.hunks {
            let mut removed: Vec<&HunkLine> = Vec::new();
            let mut added: Vec<&HunkLine> = Vec::new();
            for line in &hunk.lines {
                match line.kind {
                    LineKind::Removed if added.is_empty() => removed.push(line),
                    LineKind::Added => added.push(line),
                    // Anything else closes the run: a context line, and a
                    // removal that opens the next one.
                    _ => {
                        pairs.extend(paired(&removed, &added));
                        removed.clear();
                        added.clear();
                        if line.kind == LineKind::Removed {
                            removed.push(line);
                        }
                    }
                }
            }
            // A hunk that ends inside a run closes it where its lines run out.
            pairs.extend(paired(&removed, &added));
        }
        pairs
    }

    fn lines(&self, kind: LineKind) -> impl Iterator<Item = (Option<u32>, &str)> {
        self.diff
            .hunks
            .iter()
            .flat_map(|hunk| hunk.lines.iter())
            .filter(move |line| line.kind == kind)
            .map(|line| (line.new_line, line.text.as_str()))
    }
}

/// One line rewritten: the line that went out and the line that came in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement<'a> {
    /// The number the removed line had in the old file.
    pub old_line: u32,
    pub removed: &'a str,
    /// The number the added line has in the new file.
    pub new_line: u32,
    pub added: &'a str,
}

/// A run of removals against the run of additions that followed it, paired in
/// the order they were written. A line git could not number is not one a
/// finding can point at, so a pair missing either number is dropped.
fn paired<'a>(removed: &[&'a HunkLine], added: &[&'a HunkLine]) -> Vec<Replacement<'a>> {
    removed
        .iter()
        .zip(added)
        .filter_map(|(out, into)| {
            Some(Replacement {
                old_line: out.old_line?,
                removed: out.text.as_str(),
                new_line: into.new_line?,
                added: into.text.as_str(),
            })
        })
        .collect()
}
