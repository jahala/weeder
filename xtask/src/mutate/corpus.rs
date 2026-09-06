//! The repositories the campaign reads, and the working copy it reads them in.
//!
//! Nothing here writes to a repository weed did not make. Each source is cloned
//! into a cache directory under the temporary directory, the clone is what gets
//! checked out and mutated, and the source is only ever read from.
//!
//! The five garden repositories are the corpus calibration measures precision
//! on, so recall measures the same code. They carry no Go between them, and weed
//! judges Go, so the Go column is measured on two Go projects pinned by commit:
//! real merged history, read the same way, named in the report as what they are.

use std::path::{Path, PathBuf};

use super::git;
use super::source::Language;

/// Where a source's history comes from.
#[derive(Debug, Clone, Copy)]
pub enum Origin {
    /// A repository on this machine, cloned with its object store shared.
    Local(&'static str),
    /// A repository fetched once and pinned, so the measurement is repeatable
    /// on a machine that has never seen the garden.
    Remote {
        url: &'static str,
        rev: &'static str,
    },
}

/// One repository the campaign reads.
#[derive(Debug, Clone, Copy)]
pub struct Source {
    pub name: &'static str,
    pub origin: Origin,
    /// The languages this repository is asked for cases in.
    pub languages: &'static [Language],
}

/// The corpus. The garden five first, in the order the calibration loop lists
/// them, then the Go history that stands in for a language the garden has none
/// of.
pub const CORPUS: &[Source] = &[
    Source {
        name: "tilth",
        origin: Origin::Local("/Users/jahala/CascadeProjects/tilth"),
        languages: &[Language::Rs, Language::Py],
    },
    Source {
        name: "pleach",
        origin: Origin::Local("/Users/jahala/conductor/workspaces/pleach-v1/cayenne"),
        languages: &[Language::Ts],
    },
    Source {
        name: "tend2",
        origin: Origin::Local("/Users/jahala/conductor/workspaces/feature-map/missoula"),
        languages: &[Language::Ts],
    },
    Source {
        name: "copeca",
        origin: Origin::Local("/Users/jahala/conductor/workspaces/copeca/cancun"),
        languages: &[Language::Py],
    },
    Source {
        name: "umbel",
        origin: Origin::Local("/Users/jahala/conductor/workspaces/rctrl/master"),
        languages: &[Language::Ts],
    },
    Source {
        name: "cobra",
        origin: Origin::Remote {
            url: "https://github.com/spf13/cobra.git",
            rev: "adbc8813901bba65827259daa8e22ff94ec1f30e",
        },
        languages: &[Language::Go],
    },
    Source {
        name: "hcl",
        origin: Origin::Remote {
            url: "https://github.com/hashicorp/hcl.git",
            rev: "6abbb088cdb82416d1b3d9fcbaab29534133567a",
        },
        languages: &[Language::Go],
    },
];

impl Source {
    /// Where this source's history is on this machine, with the environment
    /// having the last word: `WEED_CORPUS_TILTH=/somewhere` moves a repository
    /// without touching the code that names it.
    pub fn located(&self) -> Option<PathBuf> {
        let key = format!("WEED_CORPUS_{}", self.name.to_uppercase());
        if let Ok(named) = std::env::var(&key) {
            return Some(PathBuf::from(named));
        }
        match self.origin {
            Origin::Local(path) => Some(PathBuf::from(path)),
            Origin::Remote { .. } => None,
        }
    }
}

/// A clone the campaign owns: checked out, mutated and restored, and never the
/// repository it was made from.
pub struct Working {
    pub root: PathBuf,
    /// The ref the history was walked from, as the report names it.
    pub reference: String,
    pub head: String,
}

/// Where the clones live between runs, so a second run costs a fetch.
pub fn cache_root() -> PathBuf {
    std::env::var("WEED_RECALL_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("weed-recall-corpus"))
}

/// Make the working clone, fetching only what is missing.
pub fn prepare(source: &Source) -> Result<Working, String> {
    let root = cache_root().join(source.name);
    std::fs::create_dir_all(cache_root())
        .map_err(|error| format!("{}: {error}", cache_root().display()))?;
    match source.origin {
        Origin::Local(_) => prepare_local(source, &root)?,
        Origin::Remote { url, rev } => prepare_remote(source, &root, url, rev)?,
    }
    let reference = reference(source, &root)?;
    let head = git::capture(&root, &["rev-parse", &reference])?;
    Ok(Working {
        root,
        reference,
        head: head.trim().to_string(),
    })
}

fn prepare_local(source: &Source, root: &Path) -> Result<(), String> {
    let Some(origin) = source.located() else {
        return Err(format!("{} has no path on this machine", source.name));
    };
    if !origin.join(".git").exists() && !origin.join("HEAD").exists() {
        return Err(format!(
            "{} is not a git repository at {}. name it with WEED_CORPUS_{}.",
            source.name,
            origin.display(),
            source.name.to_uppercase()
        ));
    }
    if !root.join(".git").exists() {
        git::run(
            &cache_root(),
            &[
                "clone",
                "--quiet",
                "--shared",
                "--no-checkout",
                &origin.to_string_lossy(),
                &root.to_string_lossy(),
            ],
        )?;
    } else {
        // A working repository moves on; the campaign reads what is there now.
        git::run(root, &["fetch", "--quiet", "--prune", "origin"])?;
    }
    Ok(())
}

fn prepare_remote(source: &Source, root: &Path, url: &str, rev: &str) -> Result<(), String> {
    if !root.join(".git").exists() {
        git::run(
            &cache_root(),
            &[
                "clone",
                "--quiet",
                "--no-checkout",
                url,
                &root.to_string_lossy(),
            ],
        )
        .map_err(|error| {
            format!(
                "{} could not be cloned from {url}: {error}. the corpus needs it once, and \
                 {} keeps it afterwards.",
                source.name,
                cache_root().display()
            )
        })?;
    }
    if git::capture(root, &["cat-file", "-t", rev]).is_err() {
        git::run(root, &["fetch", "--quiet", "origin"])?;
    }
    git::capture(root, &["cat-file", "-t", rev]).map_err(|error| {
        format!(
            "{}: the pinned commit {rev} is not in the clone: {error}",
            source.name
        )
    })?;
    Ok(())
}

/// The ref whose history is walked. A default branch that carries the work is
/// what calibration reads; a workspace checkout whose `master` holds one commit
/// is read at the branch its HEAD is on, and the report says which.
fn reference(source: &Source, root: &Path) -> Result<String, String> {
    if let Origin::Remote { rev, .. } = source.origin {
        return Ok(rev.to_string());
    }
    const FLOOR: usize = 50;
    for branch in ["main", "master"] {
        let reference = format!("refs/remotes/origin/{branch}");
        if git::capture(root, &["rev-parse", "--verify", "--quiet", &reference]).is_err() {
            continue;
        }
        let counted = git::capture(root, &["rev-list", "--count", &reference])?;
        if counted.trim().parse::<usize>().unwrap_or_default() >= FLOOR {
            return Ok(format!("origin/{branch}"));
        }
    }
    // The clone is left on a detached head between runs, so what its HEAD says
    // is not a name to walk by; `origin/HEAD` is the branch the source repository
    // is on, and it is resolved to that branch's name so the report says which.
    let default = git::capture(
        root,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )
    .map(|found| found.trim().to_string())
    .unwrap_or_default();
    let checked_out = git::capture(root, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .map(|found| found.trim().to_string())
        .unwrap_or_default();
    for candidate in [
        default.as_str(),
        checked_out.as_str(),
        "origin/HEAD",
        "HEAD",
    ] {
        if candidate.is_empty() {
            continue;
        }
        if git::capture(root, &["rev-parse", "--verify", "--quiet", candidate]).is_ok() {
            return Ok(candidate.to_string());
        }
    }
    Err(format!(
        "{}: the clone at {} names no ref to walk",
        source.name,
        root.display()
    ))
}
