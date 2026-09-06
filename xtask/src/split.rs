//! What the rules moved between two calibration runs.
//!
//! Splitting a rule in two is meant to take friction off the gate without taking
//! the finding away, and the claim is easy to make and easy to be wrong about.
//! This measurement holds the earlier run's block-level findings against this
//! one: for every file that run refused, what does this run say about the same
//! file in the same commit, and at what level.
//!
//! Nothing here knows which rule split into which. It reads the rule that
//! blocked then, the rule that speaks now, and the kind weeder gives the file, and
//! the report writes down whatever pairs it finds.

use std::collections::BTreeMap;

use weeder::core::classify::{classify_file, FileKind};

use crate::calibrate::RepoMeasurement;
use crate::first_run::FirstRun;

/// What weeder says about a file now, in the words the report uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Nothing at all is reported on the file.
    Silent,
    Note,
    Warn,
    Block,
}

impl Level {
    pub fn spelled(self) -> &'static str {
        match self {
            Level::Silent => "nothing",
            Level::Note => "note",
            Level::Warn => "warn",
            Level::Block => "block",
        }
    }

    /// SARIF's word for a level, as the measurement reads it back.
    fn of(sarif: &str) -> Option<Level> {
        match sarif {
            "error" => Some(Level::Block),
            "warning" => Some(Level::Warn),
            "note" => Some(Level::Note),
            _ => None,
        }
    }
}

/// One file the earlier run blocked, and what this run makes of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub repo: String,
    pub commit: String,
    pub path: String,
    /// The kind weeder gives the file, which is what a rule keys on.
    pub kind: String,
    /// The rule that blocked this file in the earlier run.
    pub then: String,
    /// The rule that speaks loudest about it now, and nothing where none does.
    pub now: Option<String>,
    pub level: Level,
}

impl Row {
    /// Whether this run says exactly what the first one did: the same rule, at
    /// block level still. Nothing moved, so the table leaves it out and the
    /// summary counts it.
    pub fn unchanged(&self) -> bool {
        self.level == Level::Block && self.now.as_deref() == Some(self.then.as_str())
    }
}

/// The comparison, as the report writes it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Split {
    /// The record this comparison was made against, as the report names it.
    pub record: String,
    pub rows: Vec<Row>,
    /// Findings of the earlier run on commits this run did not judge, so a
    /// shortened window cannot quietly shrink the comparison.
    pub unjudged: Vec<crate::first_run::Block>,
}

/// How many findings of one group sit on each kind of file.
type Kinds<'a> = BTreeMap<&'a str, usize>;

/// One group of the summary: how many findings moved from one rule at one level
/// to another rule at another, and how many of those are on one kind of file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    pub then: String,
    pub now: String,
    pub level: Level,
    pub count: usize,
    /// How many of them are on each kind of file, the commonest kind first.
    pub kinds: Vec<(String, usize)>,
}

impl Split {
    /// Hold the earlier run's block-level findings against this run's findings on
    /// the same commits.
    pub fn measure(first: &FirstRun, repos: &[RepoMeasurement]) -> Split {
        let by_name: BTreeMap<&str, &RepoMeasurement> = repos
            .iter()
            .map(|repo| (repo.name.as_str(), repo))
            .collect();
        let mut split = Split {
            record: first.label.clone(),
            ..Split::default()
        };
        for block in &first.blocks {
            let Some(found) = by_name
                .get(block.repo.as_str())
                .and_then(|repo| repo.recorded.get(&block.commit))
            else {
                split.unjudged.push(block.clone());
                continue;
            };
            // Several rules can speak about one file. The loudest is the one
            // that decides whether the change is still refused, and among equals
            // the lowest rule id keeps the row the same from run to run.
            let loudest = found
                .iter()
                .filter(|finding| finding.path == block.path)
                .filter_map(|finding| {
                    Level::of(&finding.level).map(|level| (level, finding.rule.clone()))
                })
                .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)));
            split.rows.push(Row {
                repo: block.repo.clone(),
                commit: block.commit.clone(),
                path: block.path.clone(),
                kind: kind_of(&block.path).to_string(),
                then: block.rule.clone(),
                now: loudest.as_ref().map(|(_, rule)| rule.clone()),
                level: loudest.map(|(level, _)| level).unwrap_or(Level::Silent),
            });
        }
        split.rows.sort_by(|left, right| {
            (&left.repo, &left.commit, &left.path, &left.then).cmp(&(
                &right.repo,
                &right.commit,
                &right.path,
                &right.then,
            ))
        });
        split
    }

    pub fn blocked_then(&self) -> usize {
        self.rows.len()
    }

    pub fn at(&self, level: Level) -> usize {
        self.rows.iter().filter(|row| row.level == level).count()
    }

    /// Every finding whose answer changed, which is what the table lists.
    pub fn changed(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter().filter(|row| !row.unchanged())
    }

    pub fn unchanged(&self) -> usize {
        self.rows.iter().filter(|row| row.unchanged()).count()
    }

    /// Every pair of rules a finding moved between, with the levels it moved
    /// from and to. A finding the same rule still makes at the same level has
    /// not moved and is not here.
    pub fn moves(&self, to: Level) -> Vec<Move> {
        let mut grouped: BTreeMap<(&str, &str), Kinds<'_>> = BTreeMap::new();
        for row in &self.rows {
            if row.level != to {
                continue;
            }
            let Some(now) = &row.now else { continue };
            let counted = grouped
                .entry((row.then.as_str(), now.as_str()))
                .or_default();
            *counted.entry(row.kind.as_str()).or_default() += 1;
        }
        grouped
            .into_iter()
            .map(|((then, now), counted)| {
                let count = counted.values().sum();
                let mut kinds: Vec<(String, usize)> = counted
                    .into_iter()
                    .map(|(kind, count)| (kind.to_string(), count))
                    .collect();
                kinds
                    .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
                Move {
                    then: then.to_string(),
                    now: now.to_string(),
                    level: to,
                    count,
                    kinds,
                }
            })
            .collect()
    }
}

/// The kind weeder gives a path, in the word the report prints. The classifier is
/// asked rather than the path read, so the report and the rules agree about what
/// a workflow is.
fn kind_of(path: &str) -> &'static str {
    match classify_file(path, "").kind {
        FileKind::Test => "test",
        FileKind::Prod => "prod",
        FileKind::Config => "config",
        FileKind::Manifest => "manifest",
        FileKind::Generated => "generated",
        FileKind::Guardrail => "guardrail",
        FileKind::Workflow => "workflow",
        FileKind::Other => "other",
    }
}
