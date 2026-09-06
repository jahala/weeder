//! The day the audited question changed, and the numbers on both sides of it.
//!
//! A report that showed only the later number would be showing a moved
//! goalpost; one that shows both, with the date, shows a corrected question.
//! The record lives in `docs/calibration/ruling-2026-09-06.toml` and the
//! report prints it from there, so the sentence is never typed.

use std::error::Error;
use std::path::Path;

use serde::Deserialize;

/// Where the record lives when no other path is given.
pub const DEFAULT_PATH: &str = "docs/calibration/ruling-2026-09-06.toml";

#[derive(Debug, Clone, Deserialize)]
pub struct Side {
    pub false_positives: usize,
    pub share_percent: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ruling {
    pub date: String,
    pub blocked: usize,
    pub before: Side,
    pub after: Side,
}

/// Read the record. A file that is not there is not an error: a repository
/// whose audited question never changed has nothing to print here.
pub fn read(path: &Path) -> Result<Option<Ruling>, Box<dyn Error>> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(None);
    };
    Ok(Some(toml::from_str(&text)?))
}

impl Ruling {
    /// The sentence the report carries beside the floor, permanently.
    #[must_use]
    pub fn sentence(&self) -> String {
        format!(
            "Both numbers stay in this file: under the three classes in use before the ruling of {} the same {} blocked commits counted {} false positives, {:.2} percent, and under it, where a blocked commit is claim-true or claim-false and `acceptable` is a label the audit never counts, they count {}, {:.2} percent. The definition changed on that date and the question was corrected, not the goalpost moved.",
            self.date,
            self.blocked,
            self.before.false_positives,
            self.before.share_percent,
            self.after.false_positives,
            self.after.share_percent,
        )
    }
}
