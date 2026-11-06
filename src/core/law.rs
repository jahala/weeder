//! What weeder does when the base of a range carries a law it cannot read (#100).
//!
//! The law comes from the base, never from the change. So a release that renames
//! or removes a `weeder.toml` key leaves every repository that used it with a
//! base this binary cannot read, and refusing to judge anything there refuses
//! the one commit that would repair it. That commit, the one that rewrites
//! `weeder.toml` and nothing else, is judged as if the base carried no law: at
//! the catalogue's own levels, with C1 still blocking the edit until a person
//! signs it. The change still supplies no law of its own, and any other change
//! over that base is still a run that never happened.

use crate::core::config::CONFIG_FILE;
use crate::core::finding::{Finding, Level, Message};

/// The id the note carries. It is not a rule: no detector reports it and
/// `[rules]` cannot turn it off, because the gap it names is in the law itself.
pub const NOTICE: &str = "LAW";

/// Whether a change that touched these paths, every path of every commit and of
/// the diff alike, may be judged over a base whose law cannot be read. It may
/// where it rewrites `weeder.toml` and touches nothing else; a change that
/// touches nothing at all is no migration either.
#[must_use]
pub fn migrates_the_law_alone<'a>(touched: impl IntoIterator<Item = &'a str>) -> bool {
    let mut touched = touched.into_iter().peekable();
    touched.peek().is_some() && touched.all(|path| path == CONFIG_FILE)
}

/// The note a run writes when it judged a migration at the catalogue's levels:
/// which law it could not read, where, and why the change was judged anyway.
#[must_use]
pub fn notice(base: &str, error: &str) -> Finding {
    Finding {
        rule: NOTICE.to_string(),
        level: Level::Note,
        path: CONFIG_FILE.to_string(),
        region: None,
        message: Message {
            what: format!(
                "{CONFIG_FILE} at {base} is a law this weeder cannot read, so this change was judged at the catalogue's own levels."
            ),
            why: format!(
                "this weeder refuses {error}. a change never supplies its own law, and this one rewrites {CONFIG_FILE} and touches nothing else: it is the repair, and refusing to judge it would leave nothing able to land."
            ),
            next: format!(
                "land it on its own, signed by a person for C1. the {CONFIG_FILE} it writes is the law every change after it is judged under."
            ),
        },
        fix: None,
        suppressed: None,
    }
}
