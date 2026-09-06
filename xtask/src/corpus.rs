//! The repositories the measurements read, and where they are.
//!
//! A corpus is a list of checkouts on the machine running the measurement, so it
//! is data rather than code: `docs/calibration/corpus.toml` names them, and
//! `--repo <name>=<path>` on the command line overrides or adds one, which is
//! what lets the suites run against a repository they built themselves.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// One repository in the corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// The name the report and the ledger key on.
    pub name: String,
    /// The checkout the measurement reads. It is never written to.
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct RawCorpus {
    repo: Vec<RawRepo>,
}

#[derive(Debug, Deserialize)]
struct RawRepo {
    name: String,
    path: String,
}

/// What a corpus could not be read as.
#[derive(Debug)]
pub enum CorpusError {
    Unreadable { path: PathBuf, message: String },
    Malformed { path: PathBuf, message: String },
    Override(String),
    Missing { name: String, path: PathBuf },
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
                "the corpus at {} is not valid: {message}. every entry is a [[repo]] with a name and a path.",
                path.display()
            ),
            CorpusError::Override(given) => write!(
                formatter,
                "--repo takes <name>=<path>, and this one is `{given}`. name the repository and where it sits."
            ),
            CorpusError::Missing { name, path } => write!(
                formatter,
                "the corpus names {name} at {}, and there is no git repository there. point it at a checkout, or take the entry out.",
                path.display()
            ),
            CorpusError::Empty => write!(
                formatter,
                "the corpus is empty, so there is no history to judge. name at least one repository."
            ),
        }
    }
}

impl std::error::Error for CorpusError {}

/// Read the corpus file, apply the overrides, and refuse a repository that is
/// not there: a calibration that quietly judged four repositories where five
/// were named would read as a calibration of five.
pub fn read(path: &Path, overrides: &[String]) -> Result<Vec<Repo>, CorpusError> {
    let text = std::fs::read_to_string(path).map_err(|error| CorpusError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let raw: RawCorpus = toml::from_str(&text).map_err(|error| CorpusError::Malformed {
        path: path.to_path_buf(),
        message: error.message().to_string(),
    })?;
    let mut repos: Vec<Repo> = raw
        .repo
        .into_iter()
        .map(|repo| Repo {
            name: repo.name,
            path: PathBuf::from(repo.path),
        })
        .collect();

    for given in overrides {
        let (name, path) = given
            .split_once('=')
            .ok_or_else(|| CorpusError::Override(given.clone()))?;
        if name.is_empty() || path.is_empty() {
            return Err(CorpusError::Override(given.clone()));
        }
        let replacement = Repo {
            name: name.to_string(),
            path: PathBuf::from(path),
        };
        match repos.iter().position(|repo| repo.name == name) {
            Some(at) => repos[at] = replacement,
            None => repos.push(replacement),
        }
    }

    if repos.is_empty() {
        return Err(CorpusError::Empty);
    }
    for repo in &repos {
        if !repo.path.join(".git").exists() && !repo.path.join("HEAD").exists() {
            return Err(CorpusError::Missing {
                name: repo.name.clone(),
                path: repo.path.clone(),
            });
        }
    }
    Ok(repos)
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
