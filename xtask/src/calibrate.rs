//! The measurement: `weed check --base <parent> --strict` over real history.
//!
//! Every commit in the window is checked out in a scratch repository and judged
//! against its parent by the same `check` face the binary runs, reading back the
//! same SARIF a pull request would upload. Nothing here decides whether a block
//! was right, that is the ledger's half; this half only counts.

use std::error::Error;
use std::path::Path;

use serde_json::Value;
use weed::faces::{check, Format};

use crate::corpus::Repo;
use crate::repo::{fingerprint, Scratch};

/// The window the loop names: up to the last two hundred commits of the default
/// branch. A branch with fewer contributes all of them.
pub const WINDOW: usize = 200;

/// Below this many commits judged, a repository is reported and not judged on
/// its own: a handful of commits cannot carry a share worth reading.
pub const REPORTED_ONLY: usize = 50;

/// One finding, as the report and the evidence dump need it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: String,
    pub level: String,
    pub path: String,
    pub line: u64,
    pub message: String,
}

/// A commit weed refused, with everything a person needs to classify it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocked {
    pub sha: String,
    pub parent: String,
    pub subject: String,
    /// The rules that fired at block level, sorted, without repeats.
    pub rules: Vec<String>,
    pub findings: Vec<Finding>,
}

/// A commit weed could not judge at all. It is neither a pass nor a block, and
/// it is never quietly dropped: a gate that could not run is its own failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub sha: String,
    pub reason: String,
}

/// What one repository's history came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMeasurement {
    pub name: String,
    /// The ref the window was taken from: the default branch its upstream names.
    pub reference: String,
    /// How many commits the window held before the roots were set aside.
    pub window: usize,
    pub judged: usize,
    pub blocked: Vec<Blocked>,
    pub warned: usize,
    pub clean: usize,
    /// Commits with no parent to be judged against.
    pub roots: usize,
    pub refusals: Vec<Refusal>,
    /// How many judged commits carried a `weed.toml` of their own. Where this is
    /// zero, every rule ran at its catalogue level.
    pub configured: usize,
}

impl RepoMeasurement {
    pub fn judged_alone(&self) -> bool {
        self.judged >= REPORTED_ONLY
    }
}

/// Judge one repository's window, leaving the repository exactly as it was
/// found. The fingerprint is taken before anything is read and again after
/// everything is, and a difference stops the run: a calibration that disturbed
/// the history it measured has measured something else.
pub fn measure(
    repo: &Repo,
    scratch_parent: &Path,
    limit: usize,
) -> Result<RepoMeasurement, Box<dyn Error>> {
    let before = fingerprint(&repo.path)?;
    let measurement = walk(repo, scratch_parent, limit);
    let after = fingerprint(&repo.path)?;
    if before != after {
        return Err(format!(
            "{} changed while it was being read, so the measurement is not of the history it names. run the calibration on a checkout nothing else is working in.",
            repo.path.display()
        )
        .into());
    }
    measurement
}

fn walk(
    repo: &Repo,
    scratch_parent: &Path,
    limit: usize,
) -> Result<RepoMeasurement, Box<dyn Error>> {
    let scratch = Scratch::fetch(scratch_parent, &repo.name, &repo.path)?;
    let reference = crate::repo::default_ref(&repo.path)?;
    let window = scratch.window(limit)?;

    let mut measurement = RepoMeasurement {
        name: repo.name.clone(),
        reference,
        window: window.len(),
        judged: 0,
        blocked: Vec::new(),
        warned: 0,
        clean: 0,
        roots: 0,
        refusals: Vec::new(),
        configured: 0,
    };

    for sha in &window {
        let Some(parent) = scratch.parent(sha)? else {
            measurement.roots += 1;
            continue;
        };
        scratch.checkout(sha)?;
        if scratch.config_file().is_some() {
            measurement.configured += 1;
        }
        measurement.judged += 1;

        let answer = check::run(&check::Request {
            cwd: scratch.path().to_path_buf(),
            base: Some(parent.clone()),
            tip: None,
            staged: false,
            scope: Vec::new(),
            strict: true,
            format: Format::Sarif,
            config: None,
            message_file: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
        });

        let findings = match results(&answer.stdout) {
            Ok(findings) => findings,
            Err(reason) => {
                measurement.refusals.push(Refusal {
                    sha: sha.clone(),
                    reason,
                });
                continue;
            }
        };
        if answer.code == weed::core::sarif::EXIT_COULD_NOT_RUN {
            measurement.refusals.push(Refusal {
                sha: sha.clone(),
                reason: answer.stderr.join(" "),
            });
            continue;
        }

        let mut rules: Vec<String> = findings
            .iter()
            .filter(|finding| finding.level == "error")
            .map(|finding| finding.rule.clone())
            .collect();
        rules.sort();
        rules.dedup();

        if rules.is_empty() {
            if findings.iter().any(|finding| finding.level == "warning") {
                measurement.warned += 1;
            } else {
                measurement.clean += 1;
            }
            continue;
        }
        measurement.blocked.push(Blocked {
            sha: sha.clone(),
            parent,
            subject: scratch.subject(sha)?,
            rules,
            findings,
        });
    }

    Ok(measurement)
}

/// The findings in a SARIF log, read the way any other consumer reads them. A
/// log weed wrote that cannot be read back is reported against the commit rather
/// than swallowed.
fn results(log: &str) -> Result<Vec<Finding>, String> {
    let log: Value = serde_json::from_str(log)
        .map_err(|error| format!("weed wrote a log that is not json: {error}"))?;
    let results = log
        .pointer("/runs/0/results")
        .and_then(Value::as_array)
        .ok_or_else(|| "weed wrote a log with no results array".to_string())?;
    results
        .iter()
        .map(|result| {
            let location = result.pointer("/locations/0/physicalLocation");
            Ok(Finding {
                rule: string(result.get("ruleId"))
                    .ok_or_else(|| "a result names no rule".to_string())?,
                level: string(result.get("level"))
                    .ok_or_else(|| "a result carries no level".to_string())?,
                path: location
                    .and_then(|location| location.pointer("/artifactLocation/uri"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                line: location
                    .and_then(|location| location.pointer("/region/startLine"))
                    .and_then(Value::as_u64)
                    .unwrap_or_default(),
                message: result
                    .pointer("/message/text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

fn string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_string)
}
