//! The check-face detectors, and the level each one reports at.
//!
//! A detector is pure: it reads a parsed diff and returns findings carrying the
//! rule's catalogue default level. `evaluate` replaces that level with the one
//! the config sets, and drops the rules the config turns off.

use crate::core::catalogue::{self, Rule};
use crate::core::config::{Config, RuleSetting};
use crate::core::diff::FileDiff;
use crate::core::finding::{Finding, Level};

pub mod g1;

type Detector = fn(&[FileDiff]) -> Vec<Finding>;

/// The detectors weed has. A catalogue rule absent from this table is not run;
/// the loop that lands its detector adds the entry here.
const DETECTORS: &[(&str, Detector)] = &[("G1", g1::evaluate)];

/// Every enabled detector's findings, in catalogue order, at the configured level.
pub fn evaluate(files: &[FileDiff], config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (id, detect) in DETECTORS {
        let Some(rule) = catalogue::rule(id) else {
            continue;
        };
        let Some(level) = configured_level(rule, config) else {
            continue;
        };
        for mut finding in detect(files) {
            finding.level = level;
            findings.push(finding);
        }
    }
    findings
}

/// The level a rule reports at, or `None` when the config turns it off.
pub fn configured_level(rule: &Rule, config: &Config) -> Option<Level> {
    match config.rules.get(rule.id) {
        Some(RuleSetting::Off) => None,
        Some(RuleSetting::Warn) => Some(Level::Warn),
        Some(RuleSetting::Block) => Some(Level::Block),
        Some(RuleSetting::On) | None => Some(rule.default_level),
    }
}
