//! What an earlier calibration run reported at block level, kept as data.
//!
//! A rule that splits in two, or loosens, changes what the gate refuses, and the
//! honest way to say by how much is to hold the two runs side by side rather
//! than to subtract one total from another. So the earlier run's block-level
//! findings are written down, one per file per commit, and a later run reads
//! them back and says what it makes of the same files.
//!
//! The record is history and is not regenerated: editing it would be editing
//! what the first run found.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// One block-level finding of the earlier run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub repo: String,
    pub commit: String,
    pub rule: String,
    pub path: String,
}

/// The earlier run, as the comparison needs it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FirstRun {
    /// The file as the report names it, inside the repository.
    pub label: String,
    pub blocks: Vec<Block>,
}

impl FirstRun {
    /// The commits the earlier run blocked in one repository. A later run keeps
    /// every finding on these so the two can be compared file by file.
    pub fn commits_of(&self, repo: &str) -> BTreeSet<String> {
        self.blocks
            .iter()
            .filter(|block| block.repo == repo)
            .map(|block| block.commit.clone())
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct Raw {
    #[serde(default)]
    block: Vec<RawBlock>,
}

#[derive(Debug, Deserialize)]
struct RawBlock {
    repo: String,
    commit: String,
    rule: String,
    path: String,
}

/// What the record could not be read as.
#[derive(Debug)]
pub enum FirstRunError {
    Unreadable { path: PathBuf, message: String },
    Malformed { path: PathBuf, message: String },
}

impl std::fmt::Display for FirstRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FirstRunError::Unreadable { path, message } => write!(
                formatter,
                "the first run's record at {} could not be read: {message}. the report says what the rules moved since that run, so it needs it.",
                path.display()
            ),
            FirstRunError::Malformed { path, message } => write!(
                formatter,
                "the first run's record at {} is not valid: {message}. every entry is a [[block]] with a repo, a commit, a rule and a path.",
                path.display()
            ),
        }
    }
}

impl std::error::Error for FirstRunError {}

/// The record a run reads when no other path is given.
pub const DEFAULT_PATH: &str = "docs/calibration/first-run.toml";

pub fn read(path: &Path, label: &str) -> Result<FirstRun, FirstRunError> {
    let text = std::fs::read_to_string(path).map_err(|error| FirstRunError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let raw: Raw = toml::from_str(&text).map_err(|error| FirstRunError::Malformed {
        path: path.to_path_buf(),
        message: error.message().to_string(),
    })?;
    let mut blocks: Vec<Block> = raw
        .block
        .into_iter()
        .map(|block| Block {
            repo: block.repo,
            commit: block.commit,
            rule: block.rule,
            path: block.path,
        })
        .collect();
    // The record is read in one order whatever order it was written in, so the
    // report it feeds writes the same bytes from the same facts.
    blocks.sort_by(|left, right| {
        (&left.repo, &left.commit, &left.path, &left.rule).cmp(&(
            &right.repo,
            &right.commit,
            &right.path,
            &right.rule,
        ))
    });
    blocks.dedup();
    Ok(FirstRun {
        label: label.to_string(),
        blocks,
    })
}
