//! R2, a public symbol has no references.
//!
//! An export is a promise to the rest of the repository. One nothing calls is
//! either dead weight or a promise somebody forgot to keep, and both are worth a
//! reader's attention. What counts as public is the language's own answer:
//! TypeScript writes `export`, Rust writes `pub`, Go capitalises the name, and
//! Python treats a leading underscore as the way to say private.
//!
//! Some exports are referenced from outside the repository by design, and
//! reporting those would teach a reader to ignore the rule. A program's entry
//! point, the root of a library, a package index and a test runner's convention
//! are all called by something weed cannot see, so none of them is reported.

use std::collections::BTreeMap;

use crate::core::classify::{FileKind, Lang};
use crate::core::config::Config;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::read::{Definition, DefinitionKind};
use crate::core::tree::{Tree, TreeFile};

use super::{code_words, Site};

/// The files whose exports are somebody else's entry point: a library's root,
/// a package index, a Python package's own namespace.
const ENTRY_FILES: &[&str] = &[
    "lib.rs",
    "index.ts",
    "index.tsx",
    "index.js",
    "index.jsx",
    "index.mjs",
    "__init__.py",
];

/// The name a program is entered through, whatever declares it.
const ENTRY_NAME: &str = "main";

/// The prefixes a Go test runner collects by, for a test written outside a
/// file the classifier already reads as a test.
const RUNNER_PREFIXES: &[&str] = &["Test", "Benchmark", "Fuzz", "Example"];

pub fn evaluate(tree: &Tree, _config: &Config) -> Vec<Finding> {
    let words = code_words(tree);
    let mut findings = Vec::new();

    for file in tree.code() {
        if file.kind() == FileKind::Test || is_entry_file(file) {
            continue;
        }
        let lines = super::code_lines(file);
        for definition in &file.outline.definitions {
            if !is_public(file, definition, &lines) || is_exempt(definition) {
                continue;
            }
            if !is_referenced(&words, file, definition) {
                findings.push(finding(file, definition));
            }
        }
    }
    findings
}

fn is_entry_file(file: &TreeFile) -> bool {
    ENTRY_FILES.contains(&file.name())
}

/// Whether the language calls this definition public.
fn is_public(file: &TreeFile, definition: &Definition, lines: &[(u32, String)]) -> bool {
    if !is_referenceable(definition.kind) {
        return false;
    }
    let declaration = lines
        .iter()
        .find(|(line, _)| *line == definition.start_line)
        .map(|(_, text)| text.trim())
        .unwrap_or_default();
    match file.lang() {
        Lang::TypeScript | Lang::JavaScript => declaration.starts_with("export"),
        Lang::Rust => declaration.starts_with("pub"),
        Lang::Go => definition
            .name
            .chars()
            .next()
            .is_some_and(|first| first.is_uppercase()),
        Lang::Python => !definition.name.starts_with('_'),
        Lang::Other => false,
    }
}

/// The kinds a repository references by name. An import is somebody else's
/// name, a module is a place, and a property belongs to whatever holds it.
fn is_referenceable(kind: DefinitionKind) -> bool {
    matches!(
        kind,
        DefinitionKind::Function
            | DefinitionKind::Class
            | DefinitionKind::Struct
            | DefinitionKind::Interface
            | DefinitionKind::TypeAlias
            | DefinitionKind::Enum
            | DefinitionKind::Constant
            | DefinitionKind::Variable
            | DefinitionKind::Export
    )
}

/// Whether the name is one a runtime or a runner reaches for on its own.
fn is_exempt(definition: &Definition) -> bool {
    let name = &definition.name;
    name == ENTRY_NAME
        || (name.starts_with("__") && name.ends_with("__"))
        || RUNNER_PREFIXES
            .iter()
            .any(|prefix| name.len() > prefix.len() && name.starts_with(prefix))
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

fn finding(file: &TreeFile, definition: &Definition) -> Finding {
    let name = &definition.name;
    Finding {
        rule: "R2".to_string(),
        level: Level::Warn,
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
        suppressed: None,
    }
}
