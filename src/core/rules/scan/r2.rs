//! R2, a public symbol has no references.
//!
//! An export is a promise to the rest of the repository. One nothing calls is
//! either dead weight or a promise somebody forgot to keep, and both are worth a
//! reader's attention. What counts as public, and which exports something
//! outside the repository calls by design, is `rules::export`'s answer, the
//! same one S4 reads.

use std::collections::BTreeMap;

use crate::core::classify::FileKind;
use crate::core::config::Config;
use crate::core::finding::{Detection, Message, Region, Stamp};
use crate::core::read::Definition;
use crate::core::rules::export;
use crate::core::tree::{Tree, TreeFile};

use super::{code_words, Site};

pub fn evaluate(tree: &Tree, _config: &Config) -> Vec<Detection> {
    let words = code_words(tree);
    let mut findings = Vec::new();

    for file in tree.code() {
        if file.kind() == FileKind::Test || export::is_entry_file(file.name()) {
            continue;
        }
        let lines = super::code_lines(file);
        for definition in &file.outline.definitions {
            if !is_public(file, definition, &lines) || export::is_exempt(definition) {
                continue;
            }
            if !is_referenced(&words, file, definition) {
                findings.push(finding(file, definition));
            }
        }
    }
    findings
}

/// Whether the language calls this definition public, read off the line it is
/// declared on.
fn is_public(file: &TreeFile, definition: &Definition, lines: &[(u32, String)]) -> bool {
    let declaration = lines
        .iter()
        .find(|(line, _)| *line == definition.start_line)
        .map(|(_, text)| text.as_str())
        .unwrap_or_default();
    export::is_public(file.lang(), definition, declaration)
}

/// Whether the repository's code writes this name anywhere outside the
/// definition itself. A function that only calls itself is called by nothing.
fn is_referenced(
    words: &BTreeMap<String, Vec<Site>>,
    file: &TreeFile,
    definition: &Definition,
) -> bool {
    words.get(&definition.name).is_some_and(|sites| {
        sites.iter().any(|site| {
            site.path != file.path
                || site.line < definition.start_line
                || site.line > definition.end_line
        })
    })
}

fn finding(file: &TreeFile, definition: &Definition) -> Detection {
    let name = &definition.name;
    Detection {
        stamp: Stamp::Rule,
        path: file.path.clone(),
        region: Some(Region {
            start_line: definition.start_line,
            end_line: definition.end_line,
        }),
        message: Message {
            what: format!("`{name}` is exported and nothing in the repository references it."),
            why: "an export nobody calls is either dead weight a reader has to understand anyway, or a promise the code was meant to keep and does not.".to_string(),
            next: format!("call `{name}`, stop exporting it, or delete it."),
        },
        fix: None,
    }
}
