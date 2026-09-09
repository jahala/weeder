//! How often a repository writes itself an allowance.
//!
//! Precision says how often weeder was wrong. It says nothing about a gate that is
//! right and routed around anyway, and the number that shows that is the rate of
//! `Weeder-allow:` trailers: rising allowances against flat true positives means
//! the gate is being gamed, or is tuned to fire where nobody agrees with it.
//!
//! Both spellings are counted, the tool's own and the one it carried before the
//! rename to weeder. A history read is not the gate. An allowance written under
//! the old name still let a finding past a gate that was running, and skipping
//! it would report a repository as more obedient than it was; honouring the old
//! name at the gate would be a different thing entirely, and weeder does not.
//!
//! The rate only means anything from the day a repository installed guard, so
//! that day is read out of the history rather than assumed: the commit that
//! first brought a hook bundle carrying guard's own marker into the tree. Before
//! that day there was no gate to allow anything past, and the count is zero.

use std::error::Error;
use std::path::Path;

use weeder::core::guard::BINARY_MARKER;
use weeder::core::suppress::parse_history_suppressions;

use crate::corpus::Repo;
use crate::repo::{fingerprint, git, Scratch};

/// When a repository put weeder's hooks in git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub sha: String,
    /// The commit date, as git wrote it: the day the gate started running.
    pub day: String,
}

/// One repository's allowance rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRate {
    pub name: String,
    /// The commit the corpus pins the history at.
    pub tip: String,
    /// Every commit reaching the pin, which is the ground the rate stands on.
    pub commits: usize,
    /// Nothing where guard was never installed.
    pub installed: Option<Installed>,
    /// Commits from the install onwards, the install itself included.
    pub commits_since: usize,
    /// Allowance trailers on those commits, in either spelling, counted one
    /// per trailer line.
    pub trailers_since: usize,
    /// Trailers written before there was a gate. They are not part of the rate,
    /// and a repository that carries any is worth saying so about.
    pub trailers_before: usize,
}

impl RepoRate {
    /// Trailers per hundred commits since guard was installed. A repository with
    /// no gate yet has written no allowance against one, so the rate is zero.
    pub fn per_hundred(&self) -> f64 {
        if self.commits_since == 0 {
            return 0.0;
        }
        self.trailers_since as f64 * 100.0 / self.commits_since as f64
    }

    /// The rate as the report writes it, with the window it was taken over.
    pub fn spelled(&self) -> String {
        match &self.installed {
            Some(installed) => format!(
                "{:.1} per 100 commits ({} in {} since {})",
                self.per_hundred(),
                self.trailers_since,
                self.commits_since,
                &installed.day[..10.min(installed.day.len())]
            ),
            None => "0.0 per 100 commits (guard not installed)".to_string(),
        }
    }
}

/// Count the allowances one repository wrote, leaving it as it was found.
pub fn measure(repo: &Repo, scratch_parent: &Path) -> Result<RepoRate, Box<dyn Error>> {
    let Some(local) = repo.local() else {
        // A url is read over upload-pack, which hands objects out and takes
        // nothing in. There is nothing on this machine for the run to disturb.
        return count(repo, scratch_parent);
    };
    let before = fingerprint(&local)?;
    let rate = count(repo, scratch_parent);
    let after = fingerprint(&local)?;
    if before != after {
        return Err(format!(
            "{} changed while it was being read, so the rate is not of the history it names. run the measurement on a checkout nothing else is working in.",
            local.display()
        )
        .into());
    }
    rate
}

fn count(repo: &Repo, scratch_parent: &Path) -> Result<RepoRate, Box<dyn Error>> {
    let scratch = Scratch::fetch(scratch_parent, &repo.name, &repo.source, &repo.tip)?;
    let history = scratch.history()?;
    let installed = install(&scratch)?;
    let at = installed
        .as_ref()
        .and_then(|installed| {
            history
                .iter()
                .position(|commit| commit.sha == installed.sha)
        })
        .unwrap_or(history.len());

    let trailers = |commits: &[crate::repo::Commit]| {
        commits
            .iter()
            .map(|commit| parse_history_suppressions(&commit.message).len())
            .sum()
    };

    Ok(RepoRate {
        name: repo.name.clone(),
        tip: repo.tip.clone(),
        commits: history.len(),
        commits_since: history.len() - at,
        trailers_since: trailers(&history[at..]),
        trailers_before: trailers(&history[..at]),
        installed,
    })
}

/// The commit that brought guard's hooks into the tree, which is the day the
/// gate started running. It is found by the marker `weeder guard install` writes
/// into every bundle it creates, so a repository that installed the hooks under
/// any directory name is still found.
fn install(scratch: &Scratch) -> Result<Option<Installed>, Box<dyn Error>> {
    let pickaxe = format!("-S{BINARY_MARKER}");
    let found = git(
        scratch.path(),
        &[
            "log",
            "--reverse",
            "--format=%H%x1f%cI",
            &pickaxe,
            "refs/heads/calibration",
        ],
    )?;
    let Some(first) = found.lines().next() else {
        return Ok(None);
    };
    let mut fields = first.split('\u{1f}');
    match (fields.next(), fields.next()) {
        (Some(sha), Some(day)) => Ok(Some(Installed {
            sha: sha.to_string(),
            day: day.to_string(),
        })),
        _ => Ok(None),
    }
}
