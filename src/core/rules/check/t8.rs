//! T8, a configuration line took a test out of the run.
//!
//! T7 is the rename that leaves what a runner collects. This is the same loss
//! reached through the settings around the file: the suite keeps its name, the
//! tree keeps the file, and one added line stops the runner from ever opening
//! it. A reviewer reading the diff sees a configuration change and a test suite
//! still sitting where it was.
//!
//! What is read is the collection the face gathered, narrowed to the lines this
//! change wrote: an exclusion somebody left in the tree years ago is `scan`'s
//! business, and this rule judges what arrived with the change.
//!
//! A line that names the file it hides, or a pattern that matches one, blocks:
//! there is nothing to work out and nothing to argue with. A line that narrows
//! what the runner reads leaves the hidden set to be worked out from everything
//! else the tree holds, so it is put in front of the reviewer rather than used
//! to stop the change.

use crate::core::finding::{Finding, Level, Message};
use crate::core::rules::check::collect::{self, Hidden};
use crate::core::rules::check::Judgement;

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    collect::one_per_test(&judged.collection.hidden)
        .iter()
        .map(finding)
        .collect()
}

fn finding(hidden: &Hidden) -> Finding {
    let what = if hidden.named {
        format!(
            "{} was taken out of the run by {}.",
            hidden.test,
            hidden.at()
        )
    } else {
        format!(
            "{} is no longer collected: {} narrows what the runner reads.",
            hidden.test,
            hidden.at()
        )
    };
    Finding {
        rule: "T8".to_string(),
        level: if hidden.named {
            Level::Block
        } else {
            Level::Warn
        },
        path: hidden.test.clone(),
        region: None,
        message: Message {
            what,
            why: "the file still looks present and no longer runs, so the suite reads as larger than it is.".to_string(),
            next: "collect the test again, delete it, or write why it is left out in a Weed-allow trailer.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
