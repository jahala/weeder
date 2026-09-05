//! The detectors, kept apart by the face that runs them.
//!
//! `check` judges a diff and `scan` judges a tree; the two read different things
//! and land on their own schedules, so each face owns its registry and neither
//! can quietly start running the other's rules.

use crate::core::catalogue::Rule;
use crate::core::config::{Config, RuleSetting};
use crate::core::finding::Level;

pub mod check;
pub mod scan;

/// The level a rule reports at, or `None` when the config turns it off.
pub fn configured_level(rule: &Rule, config: &Config) -> Option<Level> {
    match config.rules.get(rule.id) {
        Some(RuleSetting::Off) => None,
        Some(RuleSetting::Warn) => Some(Level::Warn),
        Some(RuleSetting::Block) => Some(Level::Block),
        Some(RuleSetting::On) | None => Some(rule.default_level),
    }
}
