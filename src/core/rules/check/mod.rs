//! The check-face detectors, and the level each one reports at.
//!
//! A detector is pure: it reads the [`Judgement`] the face gathered and returns
//! findings carrying the rule's catalogue default level. `evaluate` replaces
//! that level with the one the config sets, and drops the rules it turns off.

use crate::core::catalogue::{self, Rule};
use crate::core::change::Change;
use crate::core::config::Config;
use crate::core::finding::{Finding, Level};
use crate::core::read::CallerSite;
use crate::core::rules::configured_level;

pub mod c1;
pub mod c2;
pub mod c3;
pub mod collect;
pub mod d1;
pub mod d2;
pub mod g1;
pub mod g2;
pub mod generated;
pub mod idiom;
pub mod m1;
pub mod s1;
pub mod s2;
pub mod s3;
pub mod t1;
pub mod t2;
pub mod t3;
pub mod t4;
pub mod t5;
pub mod t6;
pub mod t7;
pub mod t8;
pub mod vocab;
pub mod x1;
pub mod x2;

/// Everything a check-face detector is allowed to read. The face gathers it
/// through the seams once, so a rule that needs the tree, the layers or the
/// callers of a definition stays a pure function of what it was handed.
#[derive(Debug, Clone, Copy)]
pub struct Judgement<'a> {
    /// The files the change touched, both sides of each.
    pub changes: &'a [Change],
    /// What the repository states: the rule levels, the layers, the entry points.
    pub config: &'a Config,
    /// The paths this run allows the change to touch: `--scope` where the run
    /// named one, `[scope] allow` otherwise.
    pub scope: &'a [String],
    /// Every path the repository holds, sorted. This is what a pattern is
    /// judged against, and what an import is resolved against.
    pub paths: &'a [String],
    /// Where the definitions the change touched are called from, sorted.
    pub callers: &'a [CallerSite],
    /// What the change's own lines do to what the runner collects: the suites
    /// they take out of the run, and the settings they wrote that weeder cannot
    /// read. The face reads the settings once, so the rule that judges them and
    /// the face that refuses a run it could not read see the same collection.
    pub collection: &'a collect::Collection,
}

type Detector = fn(&Judgement) -> Vec<Finding>;

/// The detectors weeder has on this face. A catalogue rule absent from this table
/// is not run; the loop that lands its detector adds the entry here.
const DETECTORS: &[(&str, Detector)] = &[
    ("T1", t1::evaluate),
    ("T2", t2::evaluate),
    ("T3", t3::evaluate),
    ("T4", t4::evaluate),
    ("T5", t5::evaluate),
    ("T6", t6::evaluate),
    ("T7", t7::evaluate),
    ("T8", t8::evaluate),
    ("M1", m1::evaluate),
    ("S1", s1::evaluate),
    ("S2", s2::evaluate),
    ("S3", s3::evaluate),
    ("D1", d1::evaluate),
    ("D2", d2::evaluate),
    ("X1", x1::evaluate),
    ("X2", x2::evaluate),
    ("C1", c1::evaluate),
    ("C2", c2::evaluate),
    ("C3", c3::evaluate),
    ("G1", g1::evaluate),
    ("G2", g2::evaluate),
];

/// Every enabled detector's findings, in catalogue order, at the configured level.
pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (id, detect) in DETECTORS {
        let Some(rule) = catalogue::rule(id) else {
            continue;
        };
        let Some(level) = configured_level(rule, judged.config) else {
            continue;
        };
        for mut finding in detect(judged) {
            finding.level = reported_at(&finding, rule, level);
            findings.push(finding);
        }
    }
    findings
}

/// The level one finding is reported at.
///
/// A detector stamps its rule's catalogue default on an ordinary finding, and
/// the config's setting takes its place: that is what `[rules]` is for. A
/// detector that reported something worse than its rule's usual case keeps the
/// level it chose, because it knows something the config does not, D1 on a
/// manifest the run was never scoped for. Turning the rule off is still how a
/// repository says it does not want to hear about it at all.
fn reported_at(finding: &Finding, rule: &Rule, configured: Level) -> Level {
    if finding.level == rule.default_level {
        configured
    } else {
        finding.level
    }
}
