//! The repositories the measurements read, and the commit each one is pinned at.
//!
//! A corpus is data rather than code: `docs/calibration/corpus.toml` names each
//! repository by something `git fetch` can read and by the full sha its window
//! ends at, and `--corpus <path>` reads a different file instead, which is what
//! lets the suites judge a history they built themselves without naming any
//! repository this one ships.
//!
//! The pin is the whole point. A source keeps moving, and a report measured
//! against "whatever the branch says today" is a different report every day; a
//! report measured against a sha is the same report until somebody edits this
//! file. Nothing here names a directory on the machine running the measurement,
//! so the corpus is fetchable from anywhere the sources are.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// One repository in the corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// The name the report and the ledger key on.
    pub name: String,
    /// Where the history is fetched from: a url, or any other thing `git fetch`
    /// takes. It is only ever read.
    pub source: String,
    /// The commit the window ends at, as a full forty-character sha.
    pub tip: String,
}

impl Repo {
    /// Where this source sits on the machine running the measurement, where it
    /// sits on one at all. A url has no path, and a fetch from it cannot write
    /// to anything; a path can be disturbed, so it is fingerprinted.
    pub fn local(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.source);
        (path.join(".git").exists() || path.join("HEAD").exists()).then_some(path)
    }
}

#[derive(Debug, Deserialize)]
struct RawCorpus {
    repo: Vec<RawRepo>,
}

#[derive(Debug, Deserialize)]
struct RawRepo {
    name: String,
    source: String,
    tip: String,
}

/// What a corpus could not be read as.
#[derive(Debug)]
pub enum CorpusError {
    Unreadable { path: PathBuf, message: String },
    Malformed { path: PathBuf, message: String },
    Tip { name: String, tip: String },
    Empty,
}

impl std::fmt::Display for CorpusError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorpusError::Unreadable { path, message } => write!(
                formatter,
                "the corpus at {} could not be read: {message}. calibration judges the repositories that file names.",
                path.display()
            ),
            CorpusError::Malformed { path, message } => write!(
                formatter,
                "the corpus at {} is not valid: {message}. every entry is a [[repo]] with a name, a source and a tip.",
                path.display()
            ),
            CorpusError::Tip { name, tip } => write!(
                formatter,
                "{name} is pinned at `{tip}`, which is not a full forty-character sha. an abbreviation can come to mean a second commit, so the pin is written out."
            ),
            CorpusError::Empty => write!(
                formatter,
                "the corpus is empty, so there is no history to judge. name at least one repository."
            ),
        }
    }
}

impl std::error::Error for CorpusError {}

/// Read the corpus file. A pin that is not a full sha is refused here rather
/// than at the fetch: the message is about the file the reader can fix, not
/// about what git said afterwards.
pub fn read(path: &Path) -> Result<Vec<Repo>, CorpusError> {
    let text = std::fs::read_to_string(path).map_err(|error| CorpusError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let raw: RawCorpus = toml::from_str(&text).map_err(|error| CorpusError::Malformed {
        path: path.to_path_buf(),
        message: error.message().to_string(),
    })?;
    let repos: Vec<Repo> = raw
        .repo
        .into_iter()
        .map(|repo| Repo {
            name: repo.name,
            source: repo.source,
            tip: repo.tip,
        })
        .collect();

    if repos.is_empty() {
        return Err(CorpusError::Empty);
    }
    for repo in &repos {
        if !is_full_sha(&repo.tip) {
            return Err(CorpusError::Tip {
                name: repo.name.clone(),
                tip: repo.tip.clone(),
            });
        }
    }
    Ok(repos)
}

fn is_full_sha(tip: &str) -> bool {
    tip.len() == 40 && tip.chars().all(|character| character.is_ascii_hexdigit())
}

/// The corpus a run reads when no other path is given.
pub const DEFAULT_PATH: &str = "docs/calibration/corpus.toml";

/// Only the named repositories, in corpus order. An empty selection is the whole
/// corpus.
pub fn select(repos: Vec<Repo>, wanted: &[String]) -> Vec<Repo> {
    if wanted.is_empty() {
        return repos;
    }
    repos
        .into_iter()
        .filter(|repo| wanted.iter().any(|name| name == &repo.name))
        .collect()
}
