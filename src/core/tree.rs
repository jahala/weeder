//! The repository as a scan rule reads it.
//!
//! `check` hands its detectors a [`crate::core::change::Change`] per changed
//! file; `scan` hands its detectors the whole tree at once, because a repository
//! question, is this cited path still there, does anything reference this
//! export, cannot be answered a file at a time. Everything a scan rule needs
//! arrives here as data the face gathered through the seams, so a detector stays
//! pure and a scan runs on a tree that never touched a disk.

use std::collections::BTreeMap;

use crate::core::classify::{Classification, FileKind, Lang};
use crate::core::read::Outline;
use crate::core::registry::Snapshot;
use crate::core::syntax::Mask;

/// One file of the tree, read once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeFile {
    /// Repository-relative, separated by `/`.
    pub path: String,
    /// The file's text, or `None` where what it holds is not text. weed judges
    /// lines, and bytes that are not text carry none.
    pub content: Option<String>,
    pub classification: Option<Classification>,
    /// What the file defines, empty where it defines nothing or where weed has
    /// no grammar for it.
    pub outline: Outline,
}

impl TreeFile {
    /// The file's text, or nothing at all where it holds none.
    #[must_use]
    pub fn text(&self) -> &str {
        self.content.as_deref().unwrap_or_default()
    }

    #[must_use]
    pub fn lang(&self) -> Lang {
        self.classification
            .as_ref()
            .map_or(Lang::Other, |classification| classification.lang)
    }

    #[must_use]
    pub fn kind(&self) -> FileKind {
        self.classification
            .as_ref()
            .map_or(FileKind::Other, |classification| classification.kind)
    }

    /// Whether the file is written in a language weed reads.
    #[must_use]
    pub fn is_code(&self) -> bool {
        self.lang() != Lang::Other
    }

    /// The file read as the language it is written in, so a rule can tell the
    /// program from what the program merely says.
    #[must_use]
    pub fn mask(&self) -> Mask {
        Mask::of(self.lang(), self.text())
    }

    /// The last segment of the path.
    #[must_use]
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// The file's extension, without the dot, lowercased. `None` where the name
    /// carries none, and where the only dot opens the name, a dotfile has no
    /// extension.
    #[must_use]
    pub fn extension(&self) -> Option<String> {
        let name = self.name();
        let (stem, extension) = name.rsplit_once('.')?;
        if stem.is_empty() || extension.is_empty() {
            return None;
        }
        Some(extension.to_ascii_lowercase())
    }
}

/// What one command accepts, as its own help printed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandListing {
    /// The command and the subcommands that reach this listing, in order:
    /// `["weed"]`, then `["weed", "guard"]`, then `["weed", "guard", "install"]`.
    pub path: Vec<String>,
    /// The subcommands this listing offers, by name.
    pub subcommands: Vec<String>,
    /// Every flag spelling this listing prints, leading dashes included.
    pub flags: Vec<String>,
}

/// The repository as it is, and what the face had to leave the tree to learn.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tree {
    pub files: Vec<TreeFile>,
    /// When each line of a file was last touched, seconds since the epoch, in
    /// line order. Only the files a rule asked about are here: blaming a whole
    /// repository to date a handful of comments would cost more than the scan.
    pub blame: BTreeMap<String, Vec<i64>>,
    /// The moment the scan started, seconds since the epoch. A rule that
    /// measures an age is asking about now, and now is I/O like any other.
    pub now: i64,
    /// The help listings of the commands `[docs] commands` named, walked from
    /// each command down through its subcommands.
    pub commands: Vec<CommandListing>,
    /// What the registries last said, as the repository committed it. A scan
    /// never asks them itself.
    pub snapshot: Snapshot,
}

impl Tree {
    /// Whether the tree holds a file at exactly this path.
    #[must_use]
    pub fn holds(&self, path: &str) -> bool {
        self.files.iter().any(|file| file.path == path)
    }

    /// Whether the tree holds anything under this path, so a cited directory
    /// resolves as well as a cited file.
    #[must_use]
    pub fn holds_under(&self, prefix: &str) -> bool {
        let prefix = format!("{}/", prefix.trim_end_matches('/'));
        self.files.iter().any(|file| file.path.starts_with(&prefix))
    }

    /// Whether any file in the tree carries this name, wherever it sits. A doc
    /// that cites a bare filename is citing the file, not a place.
    #[must_use]
    pub fn holds_name(&self, name: &str) -> bool {
        self.files.iter().any(|file| file.name() == name)
    }

    /// Every extension the tree actually uses. A doc citing `weed.toml` is
    /// citing a file when the repository is written in files like it, and is
    /// citing something else, a member, a marker, when it is not. The
    /// vocabulary comes from the tree so no list of known extensions has to be
    /// kept anywhere.
    #[must_use]
    pub fn extensions(&self) -> std::collections::BTreeSet<String> {
        self.files.iter().filter_map(TreeFile::extension).collect()
    }

    /// The files written in a language weed reads.
    pub fn code(&self) -> impl Iterator<Item = &TreeFile> {
        self.files.iter().filter(|file| file.is_code())
    }

    /// The listing a command path reaches, or `None` where no command was
    /// walked that far.
    #[must_use]
    pub fn listing(&self, path: &[String]) -> Option<&CommandListing> {
        self.commands.iter().find(|listing| listing.path == path)
    }
}
