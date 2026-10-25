//! The bite-face detector, and the level it reports at.
//!
//! One rule lives here, and it reads a [`Trial`] the face ran rather than a
//! diff or a tree. The shape is the one the other two faces keep: the detector
//! is pure and stamps its rule's level, and `evaluate` names the rule and puts
//! the configured level in its place, or drops the rule where a repository
//! turned it off.

use crate::core::bite::Trial;
use crate::core::catalogue;
use crate::core::config::Config;
use crate::core::finding::{Detection, Finding};
use crate::core::rules::report;

pub mod b1;

type Detector = fn(&Trial) -> Vec<Detection>;

/// The detectors weeder has on this face.
const DETECTORS: &[(&str, Detector)] = &[("B1", b1::evaluate)];

/// Every enabled detector's findings, at the configured level.
pub fn evaluate(trial: &Trial, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (id, detect) in DETECTORS {
        let Some(rule) = catalogue::rule(id) else {
            continue;
        };
        findings.extend(report(rule, config, || detect(trial)));
    }
    findings
}
