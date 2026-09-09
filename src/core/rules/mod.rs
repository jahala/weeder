//! The detectors, kept apart by the face that runs them.
//!
//! `check` judges a diff, `scan` judges a tree and `bite` judges what a test
//! command did with two states of one; the three read different things and land
//! on their own schedules, so each face owns its registry and none of them can
//! quietly start running another's rules.

use crate::core::catalogue::Rule;
use crate::core::config::{Config, RuleSetting};
use crate::core::finding::{Finding, Level};

pub mod bite;
pub mod check;
pub mod scan;

/// The level one finding is reported at.
///
/// A detector stamps its rule's catalogue default on an ordinary finding, and
/// the config's setting takes its place: that is what `[rules]` is for. A
/// detector that reported at a level of its own keeps it, because it knows
/// something the config does not: D1 louder on a manifest the run was never
/// scoped for, R1 quieter on a name no paragraph pinned to a place. Turning the
/// rule off is still how a repository says it does not want to hear about it at
/// all.
pub fn reported_at(finding: &Finding, rule: &Rule, configured: Level) -> Level {
    if finding.level == rule.default_level {
        configured
    } else {
        finding.level
    }
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
