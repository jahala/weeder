//! The check-face detectors, and the level each one reports at.
//!
//! A detector is pure: it reads the changed files the face gathered and returns
//! findings carrying the rule's catalogue default level. `evaluate` replaces
//! that level with the one the config sets, and drops the rules it turns off.

use crate::core::catalogue::{self};
use crate::core::change::Change;
use crate::core::config::Config;
use crate::core::finding::Finding;
use crate::core::rules::configured_level;

pub mod c1;
pub mod g1;
pub mod m1;
pub mod s1;
pub mod t1;
pub mod t2;
pub mod t3;
pub mod t4;
pub mod t5;
pub mod t6;
pub mod t7;
pub mod vocab;
pub mod x1;

type Detector = fn(&[Change]) -> Vec<Finding>;

/// The detectors weed has on this face. A catalogue rule absent from this table
/// is not run; the loop that lands its detector adds the entry here.
const DETECTORS: &[(&str, Detector)] = &[
    ("T1", t1::evaluate),
    ("T2", t2::evaluate),
    ("T3", t3::evaluate),
    ("T4", t4::evaluate),
    ("T5", t5::evaluate),
    ("T6", t6::evaluate),
    ("T7", t7::evaluate),
    ("M1", m1::evaluate),
    ("S1", s1::evaluate),
    ("X1", x1::evaluate),
    ("C1", c1::evaluate),
    ("G1", g1::evaluate),
];

/// Every enabled detector's findings, in catalogue order, at the configured level.
pub fn evaluate(changes: &[Change], config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (id, detect) in DETECTORS {
        let Some(rule) = catalogue::rule(id) else {
            continue;
        };
        let Some(level) = configured_level(rule, config) else {
            continue;
        };
        for mut finding in detect(changes) {
            finding.level = level;
            findings.push(finding);
        }
    }
    findings
}
