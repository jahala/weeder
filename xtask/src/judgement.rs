//! The classifications, which are a person's judgement and not a measurement.
//!
//! weed counts what it blocked; whether a block was right is read by whoever ran
//! the calibration, one line per commit, and written down here. The report puts
//! the two together, and the bar checks the arithmetic over them. Nothing in
//! this file decides a classification, and nothing outside it may invent one: a
//! blocked commit nobody has classified is counted as a false positive, so the
//! bar can only be reached by looking at the diffs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// What a block turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    /// The change really did what the rule says it did.
    TruePositive,
    /// The rule fired as designed on a change a person should see, and the
    /// change turned out to be fine. The friction is the rule working.
    Acceptable,
    /// The rule fired on something it was not written to catch.
    FalsePositive,
}

impl Classification {
    /// How the report spells it, and how the bar reads it back.
    pub fn spelled(self) -> &'static str {
        match self {
            Classification::TruePositive => "true positive",
            Classification::Acceptable => "acceptable",
            Classification::FalsePositive => "false positive",
        }
    }
}

/// One commit's judgement: what the block was, why, and where a rule in a
/// multi-rule block deserves a different answer from the commit as a whole.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Judgement {
    pub repo: String,
    pub sha: String,
    pub classification: Classification,
    /// One line: what the rule saw, and why that is the answer.
    pub reasoning: String,
    /// A rule that deserves its own answer inside this commit. A rule not named
    /// here takes the commit's classification and its reasoning.
    #[serde(default)]
    pub rules: BTreeMap<String, RuleJudgement>,
}

/// One rule's own answer inside a block that fired several.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuleJudgement {
    pub classification: Classification,
    pub reasoning: String,
}

impl Judgement {
    /// The classification this commit's block gives one rule.
    pub fn for_rule(&self, rule: &str) -> Classification {
        self.rules
            .get(rule)
            .map(|judgement| judgement.classification)
            .unwrap_or(self.classification)
    }

    /// Why, for one rule of the block. A rule with its own answer has its own
    /// line; every other rule is answered by the commit's.
    pub fn reason_for_rule(&self, rule: &str) -> &str {
        self.rules
            .get(rule)
            .map(|judgement| judgement.reasoning.as_str())
            .unwrap_or(self.reasoning.as_str())
    }
}

#[derive(Debug, Deserialize)]
struct RawLedger {
    #[serde(default)]
    commit: Vec<Judgement>,
}

/// Every judgement, keyed by repository and full commit sha.
#[derive(Debug, Default)]
pub struct Ledger {
    entries: BTreeMap<(String, String), Judgement>,
}

#[derive(Debug)]
pub enum LedgerError {
    Unreadable { path: PathBuf, message: String },
    Malformed { path: PathBuf, message: String },
    Blank { repo: String, sha: String },
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::Unreadable { path, message } => write!(
                formatter,
                "the judgements at {} could not be read: {message}. an unread ledger would classify every block as a false positive.",
                path.display()
            ),
            LedgerError::Malformed { path, message } => write!(
                formatter,
                "the judgements at {} are not valid: {message}. each entry is a [[commit]] with repo, sha, classification and reasoning.",
                path.display()
            ),
            LedgerError::Blank { repo, sha } => write!(
                formatter,
                "the judgement for {repo} {sha} carries no reasoning. a classification without a reason cannot be reviewed, so it is not accepted."
            ),
        }
    }
}

impl std::error::Error for LedgerError {}

impl Ledger {
    /// Read the ledger. A file that is not there is an empty ledger: the first
    /// calibration of a corpus has classified nothing yet, and the report says so
    /// commit by commit.
    pub fn read(path: &Path) -> Result<Ledger, LedgerError> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Ledger::default())
            }
            Err(error) => {
                return Err(LedgerError::Unreadable {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })
            }
        };
        let raw: RawLedger = toml::from_str(&text).map_err(|error| LedgerError::Malformed {
            path: path.to_path_buf(),
            message: error.message().to_string(),
        })?;
        let mut entries = BTreeMap::new();
        for judgement in raw.commit {
            if judgement.reasoning.trim().is_empty()
                || judgement
                    .rules
                    .values()
                    .any(|rule| rule.reasoning.trim().is_empty())
            {
                return Err(LedgerError::Blank {
                    repo: judgement.repo,
                    sha: judgement.sha,
                });
            }
            entries.insert((judgement.repo.clone(), judgement.sha.clone()), judgement);
        }
        Ok(Ledger { entries })
    }

    pub fn get(&self, repo: &str, sha: &str) -> Option<&Judgement> {
        self.entries.get(&(repo.to_string(), sha.to_string()))
    }
}
