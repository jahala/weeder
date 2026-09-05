//! The scan-face detectors, and the level each one reports at.
//!
//! A detector is pure: it reads the tree the face gathered and returns findings
//! carrying the rule's catalogue default level. `evaluate` replaces that level
//! with the one the config sets, and drops the rules it turns off. Every scan
//! rule reports a warning, `scan` never blocks, so what the config decides
//! here is whether a rule runs at all.

use std::collections::{BTreeMap, BTreeSet};

use crate::core::catalogue;
use crate::core::config::Config;
use crate::core::finding::Finding;
use crate::core::rules::configured_level;
use crate::core::syntax;
use crate::core::tree::{Tree, TreeFile};

pub mod r1;
pub mod r2;
pub mod r3;
pub mod r4;

type Detector = fn(&Tree, &Config) -> Vec<Finding>;

/// The detectors weed has on this face, in catalogue order.
const DETECTORS: &[(&str, Detector)] = &[
    ("R1", r1::evaluate),
    ("R2", r2::evaluate),
    ("R3", r3::evaluate),
    ("R4", r4::evaluate),
];

/// Every enabled detector's findings, in catalogue order, at the configured
/// level. `only` narrows the run to the rule ids a caller named; an empty list
/// runs the whole face.
pub fn evaluate(tree: &Tree, config: &Config, only: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (id, detect) in DETECTORS {
        if !only.is_empty() && !only.iter().any(|wanted| wanted == id) {
            continue;
        }
        let Some(rule) = catalogue::rule(id) else {
            continue;
        };
        let Some(level) = configured_level(rule, config) else {
            continue;
        };
        for mut finding in detect(tree, config) {
            finding.level = level;
            findings.push(finding);
        }
    }
    findings
}

/// Where a word was written: the file, and the 1-based line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    pub path: String,
    pub line: u32,
}

/// Every identifier the tree's code carries, and where each one is written.
///
/// Only the code counts. A name in a comment is a mention and a name in a
/// string is a message; neither reaches the program, so neither keeps a symbol
/// alive and neither proves a cited symbol still exists.
#[must_use]
pub fn code_words(tree: &Tree) -> BTreeMap<String, Vec<Site>> {
    let mut found: BTreeMap<String, Vec<Site>> = BTreeMap::new();
    for file in tree.code() {
        for (line, text) in code_lines(file) {
            for word in syntax::words(&text) {
                found.entry(word.text.to_string()).or_default().push(Site {
                    path: file.path.clone(),
                    line,
                });
            }
        }
    }
    found
}

/// Each 1-based line of a file with everything that is not code blanked.
#[must_use]
pub fn code_lines(file: &TreeFile) -> Vec<(u32, String)> {
    let mask = file.mask();
    (1..=file.text().lines().count() as u32)
        .map(|line| (line, mask.code(line)))
        .collect()
}

/// Every name the tree declares, at any depth.
#[must_use]
pub fn declared_names(tree: &Tree) -> BTreeSet<String> {
    tree.files
        .iter()
        .flat_map(|file| file.outline.flatten())
        .map(|definition| definition.name.clone())
        .collect()
}
