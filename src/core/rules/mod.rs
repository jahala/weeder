//! The detectors, kept apart by the face that runs them.
//!
//! `check` judges a diff, `scan` judges a tree and `bite` judges what a test
//! command did with two states of one; the three read different things and land
//! on their own schedules, so each face owns its registry and none of them can
//! quietly start running another's rules.

use crate::core::catalogue::Rule;
use crate::core::config::{Config, RuleSetting};
use crate::core::finding::{Detection, Finding, Level, Stamp};

pub mod bite;
pub mod check;
pub mod scan;

/// What one rule's detector found, as the findings it is reported as. The
/// detector runs only where the config leaves its rule on.
pub fn report(
    rule: &Rule,
    config: &Config,
    detect: impl FnOnce() -> Vec<Detection>,
) -> Vec<Finding> {
    if configured_level(rule, config).is_none() {
        return Vec::new();
    }
    detect()
        .into_iter()
        .filter_map(|detection| {
            reported_at(detection.stamp, rule, config)
                .map(|level| detection.reported(rule.id, level))
        })
        .collect()
}

/// The level one finding is reported at, or `None` when the config turns its
/// rule off.
///
/// A finding stamped with its rule's level reports at the catalogue default,
/// or at the level `[rules]` writes for the rule: that is what `[rules]` is
/// for. A detector that asked for a level of its own keeps it, because it knows
/// something the config does not: D1 louder on a manifest the run was never
/// scoped for, R1 quieter on a name no paragraph pinned to a place. A level
/// the repository wrote still caps it, so `D1 = "warn"` blocks nothing.
pub fn reported_at(stamp: Stamp, rule: &Rule, config: &Config) -> Option<Level> {
    let level = configured_level(rule, config)?;
    Some(match (stamp, config.rules.get(rule.id)) {
        (Stamp::Rule, _) => level,
        (Stamp::Override(own), Some(RuleSetting::Warn | RuleSetting::Block)) => own.at_most(level),
        (Stamp::Override(own), _) => own,
    })
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
