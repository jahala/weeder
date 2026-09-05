//! A changed file, and everything a rule reads about it.
//!
//! A detector is pure, so whatever it needs about a file has to arrive as data.
//! That is a [`Change`]: the hunks git wrote, and each side of the change as the
//! face gathered it — the text, what the path classifies as, the tests the file
//! declares and what it defines. The face reads those through the seams; core
//! only ever looks at them, which is what keeps a rule runnable on a diff that
//! never touched a disk.

use crate::core::classify::{Classification, FileKind, Lang};
use crate::core::diff::{ChangeKind, FileDiff, LineKind};
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

    fn lines(&self, kind: LineKind) -> impl Iterator<Item = (Option<u32>, &str)> {
        self.diff
            .hunks
            .iter()
            .flat_map(|hunk| hunk.lines.iter())
            .filter(move |line| line.kind == kind)
            .map(|line| (line.new_line, line.text.as_str()))
    }
}
