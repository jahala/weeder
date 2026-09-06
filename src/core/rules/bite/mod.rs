//! The bite-face detector, and the level it reports at.
//!
//! One rule lives here, and it reads a [`Trial`] the face ran rather than a
//! diff or a tree. The shape is the one the other two faces keep: the detector
//! is pure and stamps its catalogue default, and `evaluate` puts the configured
//! level in its place, or drops the rule where a repository turned it off.

use crate::core::bite::Trial;
use crate::core::catalogue;
use crate::core::config::Config;
use crate::core::finding::Finding;
use crate::core::rules::configured_level;

pub mod b1;

type Detector = fn(&Trial) -> Vec<Finding>;

/// The detectors weed has on this face.
const DETECTORS: &[(&str, Detector)] = &[("B1", b1::evaluate)];

/// Every enabled detector's findings, at the configured level.
pub fn evaluate(trial: &Trial, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (id, detect) in DETECTORS {
        let Some(rule) = catalogue::rule(id) else {
            continue;
        };
        let Some(level) = configured_level(rule, config) else {
            continue;
        };
        for mut finding in detect(trial) {
            finding.level = level;
            findings.push(finding);
        }
    }
    findings
}
