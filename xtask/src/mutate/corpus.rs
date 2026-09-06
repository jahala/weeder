//! The repositories the campaign reads, and the working copy it reads them in.
//!
//! The corpus is the one calibration judges. `docs/calibration/corpus.toml`
//! names each repository by something `git fetch` can read and by the full sha
//! its window ends at, and this campaign walks the window that ends at that
//! sha. The pin is what makes recall a measurement rather than a reading: a
//! branch keeps moving, and a campaign that plants its cases in "whatever the
//! branch says today" reports a different miss list tomorrow over what is
//! nominally one corpus.
//!
//! The garden five write no Go between them and weed judges Go, so
//! `docs/calibration/corpus-go.toml` pins two Go projects the same way, read
//! exactly as the five are. Any other corpus file is read with `--corpus`,
//! which is how the suites measure a history they built themselves.
//!
//! Nothing here writes to a source repository: a fetch runs `upload-pack` over
//! there and takes objects away, and every checkout and every mutation happens
//! in a clone under the cache directory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::git;
use super::source::Language;
use crate::corpus::Repo;

/// The Go history recall adds to the corpus calibration judges, pinned the same
/// way and named in the report as what it is.
pub const GO_PATH: &str = "docs/calibration/corpus-go.toml";

/// The one branch a clone carries, so the walk never has to ask which of the
/// source's branches it landed on: it walks the pin.
const BRANCH: &str = "refs/heads/recall";

/// A language counts as one a repository writes where it holds at least a
/// twentieth of the repository's source files. A stray file of somebody else's
/// language is not a history to measure recall in, and asking a repository for
/// cases in it would spend the run's quota on commits that offer no site.
const SHARE: usize = 20;

/// How many places of its window one repository is walked from at once.
///
/// A case costs a checkout and a run of the binary, and a campaign of thousands
/// of them spends most of its time waiting rather than computing, so the window
/// is walked from several places at once, each in a working copy of its own.
/// The number is fixed rather than read off the machine: the cases a run plants
/// are the same everywhere, and a count that followed the cores would make the
/// report a fact about the laptop it was taken on.
pub const SLICES: usize = 4;

/// Read every corpus file named, in order. A name that appears twice is
/// refused: two entries under one name would share a clone and a row.
pub fn read(paths: &[PathBuf]) -> Result<Vec<Repo>, String> {
    let mut repos: Vec<Repo> = Vec::new();
    for path in paths {
        for repo in crate::corpus::read(path).map_err(|error| error.to_string())? {
            if repos.iter().any(|held| held.name == repo.name) {
                return Err(format!(
                    "{} is named by two corpus entries. one name is one repository, one clone and \
                     one row.",
                    repo.name
                ));
            }
            repos.push(repo);
        }
    }
    Ok(repos)
}

/// A clone the campaign owns: fetched at the pin, checked out, mutated and
/// restored, and never the repository it was fetched from.
pub struct Working {
    pub name: String,
    /// Where the history was fetched from, as the corpus names it.
    pub source: String,
    /// The commit the window ends at.
    pub tip: String,
    /// The languages this repository writes, read off the tree at the pin.
    pub languages: Vec<Language>,
    pub root: PathBuf,
}

/// Where the clones live between runs, so a second run costs a fetch of what
/// the pin needs and nothing more.
pub fn cache_root() -> PathBuf {
    std::env::var("WEED_RECALL_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("weed-recall-corpus"))
}

/// Make the working clone, fetching only what the pin is missing.
pub fn prepare(repo: &Repo) -> Result<Working, String> {
    prepare_in(&cache_root(), repo)
}

/// Make the working clone under a caller-owned root. The campaign uses the
/// shared recall cache; one-case audit replay uses a private temp root so a
/// packet cannot inherit a half-restored campaign checkout.
pub fn prepare_in(cache: &Path, repo: &Repo) -> Result<Working, String> {
    let root = cache.join(&repo.name);
    std::fs::create_dir_all(&root).map_err(|error| format!("{}: {error}", root.display()))?;
    if !root.join(".git").exists() {
        git::run(&root, &["init", "--quiet"])?;
    }
    let commit = format!("{}^{{commit}}", repo.tip);
    if git::run(&root, &["cat-file", "-e", &commit]).is_err() {
        git::run(
            &root,
            &["fetch", "--quiet", "--no-tags", &repo.source, &repo.tip],
        )
        .map_err(|error| {
            format!(
                "{}: {} would not hand over {}. the corpus pins a commit, and a source that no \
                 longer carries it cannot be measured. {error}",
                repo.name, repo.source, repo.tip
            )
        })?;
    }
    git::run(&root, &["update-ref", BRANCH, &repo.tip])?;
    let head = git::capture(&root, &["rev-parse", BRANCH])?
        .trim()
        .to_string();
    if head != repo.tip {
        return Err(format!(
            "{}: the clone is at {head} and the corpus pins {}",
            repo.name, repo.tip
        ));
    }
    let languages = languages(&root, &repo.tip)?;
    Ok(Working {
        name: repo.name.clone(),
        source: repo.source.clone(),
        tip: repo.tip.clone(),
        languages,
        root,
    })
}

/// The working copy one slice of a repository's window is walked in: a clone of
/// the clone, sharing its objects, so a checkout in one slice is nothing to the
/// others. It is made once and kept, like the clone it comes from.
pub fn slice(working: &Working, index: usize) -> Result<PathBuf, String> {
    if index == 0 {
        return Ok(working.root.clone());
    }
    let root = cache_root().join(format!("{}-{index}", working.name));
    if !root.join(".git").exists() {
        git::run(
            &cache_root(),
            &[
                "clone",
                "--quiet",
                "--shared",
                "--no-checkout",
                &working.root.to_string_lossy(),
                &root.to_string_lossy(),
            ],
        )?;
    }
    git::run(&root, &["update-ref", BRANCH, &working.tip])?;
    Ok(root)
}

/// The languages a repository writes, counted off the tree at the pin rather
/// than declared beside its name: a corpus is a list of repositories, and what
/// each one is written in is a fact about it that the pin fixes.
fn languages(root: &Path, tip: &str) -> Result<Vec<Language>, String> {
    Ok(spoken(&git::lines(
        root,
        &["ls-tree", "-r", "--name-only", tip],
    )?))
}

/// The languages a tree of paths is written in.
fn spoken(paths: &[String]) -> Vec<Language> {
    let mut counted: BTreeMap<Language, usize> = BTreeMap::new();
    for path in paths {
        for lang in Language::ALL {
            if lang.owns(path) {
                *counted.entry(lang).or_default() += 1;
            }
        }
    }
    let held: usize = counted.values().sum();
    Language::ALL
        .into_iter()
        .filter(|lang| match counted.get(lang) {
            Some(files) => *files > 0 && files * SHARE >= held,
            None => false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(each: &[(&str, usize)]) -> Vec<String> {
        let mut written = Vec::new();
        for (extension, count) in each {
            for number in 0..*count {
                written.push(format!("src/{extension}/file{number}.{extension}"));
            }
        }
        written
    }

    /// A repository is asked for cases in the languages it is written in, and
    /// what it is written in is counted rather than declared beside its name.
    #[test]
    fn a_repository_is_read_for_every_language_it_writes() {
        assert_eq!(
            spoken(&paths(&[("rs", 91), ("py", 36)])),
            vec![Language::Py, Language::Rs]
        );
    }

    /// One file of somebody else's language is not a history to measure recall
    /// in, and a quota spent on it is a quota spent on commits with no site.
    #[test]
    fn a_stray_file_is_not_a_language_the_repository_writes() {
        assert_eq!(
            spoken(&paths(&[("ts", 119), ("go", 1)])),
            vec![Language::Ts]
        );
    }

    #[test]
    fn a_tree_with_no_source_in_it_is_read_for_nothing() {
        assert!(spoken(&paths(&[("md", 12)])).is_empty());
    }
}
