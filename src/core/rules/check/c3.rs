//! C3, a workflow was changed.
//!
//! A workflow says how the checks are run on a server. It decides less than a
//! guardrail does and is rewritten far more often, a step reordered, a runner
//! bumped, a cache key changed, so weed names the change and lets it through:
//! this is a warning for the person reading the pull request, not a wall.
//!
//! One repository's by-law is another's constitution. Where the workflow is
//! what publishes, `[guardrails] paths` in `weed.toml` names it, and weed
//! reports that path at block level instead. Only the paths the repository
//! wrote down move; the rest stay warnings.

use crate::core::change::Change;
use crate::core::classify::{classify_file, FileKind};
use crate::core::config::Config;
use crate::core::finding::{Finding, Level, Message};
use crate::core::glob;
use crate::core::rules::check::Judgement;

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let changes = judged.changes;
    changes
        .iter()
        .filter_map(|change| workflow(change, judged.config))
        .collect()
}

fn workflow(change: &Change, config: &Config) -> Option<Finding> {
    let path = change.path()?;
    if classify_file(path, "").kind != FileKind::Workflow {
        return None;
    }
    let promoted = glob::matches_any(&config.guardrail_paths, path);
    Some(Finding {
        rule: "C3".to_string(),
        level: if promoted { Level::Block } else { Level::Warn },
        path: path.to_string(),
        region: None,
        message: message(promoted),
        fix: None,
        suppressed: None,
    })
}

/// What the finding says, in the tier the repository holds the path at.
fn message(promoted: bool) -> Message {
    if promoted {
        return Message {
            what: "a workflow this repository holds as a guardrail was changed.".to_string(),
            why: "`[guardrails] paths` names this file, so what it runs is law here rather than routine.".to_string(),
            next: "revert it, or land the change to the workflow on its own so a person reads it.".to_string(),
        };
    }
    Message {
        what: "a workflow was changed.".to_string(),
        why: "this file decides which checks a server runs, so a change here can quietly stop running one.".to_string(),
        next: "read the diff against what the workflow used to run, and say in the change why a step moved.".to_string(),
    }
}
