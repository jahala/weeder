//! R5, a test file the configuration never collects.
//!
//! T8 judges the line that takes a suite out of the run, which helps only where
//! somebody is watching a change go past. A repository that arrived with the
//! exclusion already in it has no change to stop, and its suite count still
//! reads as though every file in the tree ran. So the same collector is read
//! against the tree as it sits, and every suite the runner will not open is
//! named once, whoever wrote the line and whenever they wrote it.
//!
//! Nothing here blocks: a scan reports the state a repository is in, and no
//! reading of that state is a reason to stop a change.

use crate::core::config::Config;
use crate::core::finding::{Finding, Level, Message};
use crate::core::rules::check::collect::{self, File, Hidden};
use crate::core::tree::Tree;

pub fn evaluate(tree: &Tree, _config: &Config) -> Vec<Finding> {
    let read: Vec<File<'_>> = tree
        .files
        .iter()
        .map(|file| File {
            path: &file.path,
            kind: file.kind(),
            lang: file.lang(),
            mask: file.mask(),
        })
        .collect();
    let paths: Vec<String> = tree.files.iter().map(|file| file.path.clone()).collect();
    let collection = collect::uncollected(&read, &paths);
    collect::one_per_test(&collection.hidden)
        .iter()
        .map(finding)
        .collect()
}

fn finding(hidden: &Hidden) -> Finding {
    Finding {
        rule: "R5".to_string(),
        level: Level::Warn,
        path: hidden.test.clone(),
        region: None,
        message: Message {
            what: format!(
                "the runner never collects {}: {} keeps it out.",
                hidden.test,
                hidden.at()
            ),
            why: "a suite nobody runs reports nothing, and the tests inside it still read as present.".to_string(),
            next: "collect it again, delete it, or say in the configuration why it is kept out.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
