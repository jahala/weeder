//! What a language calls exported, and which exports are called from outside
//! the repository by design.
//!
//! Two rules ask. R2 reports an export nothing references, and S4 one only its
//! tests call. Both need the same two answers, so they are written here once.
//!
//! What counts as public is the language's own answer: TypeScript writes
//! `export`, Rust writes `pub`, Go capitalises the name, and Python treats a
//! leading underscore as the way to say private.
//!
//! Some exports are called by something weeder cannot see. A program's entry
//! point, the root of a library, a package index and a test runner's convention
//! all are, so neither rule judges them.

use crate::core::classify::Lang;
use crate::core::read::{Definition, DefinitionKind};

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

/// Whether a file of this name is one whose exports are an entry point.
#[must_use]
pub fn is_entry_file(name: &str) -> bool {
    ENTRY_FILES.contains(&name)
}

/// Whether the language calls this definition public. `declaration` is the
/// code of the line the definition starts on, with its comments and literals
/// blanked.
#[must_use]
pub fn is_public(lang: Lang, definition: &Definition, declaration: &str) -> bool {
    if !is_referenceable(definition.kind) {
        return false;
    }
    let declaration = declaration.trim();
    match lang {
        Lang::TypeScript | Lang::JavaScript => declaration.starts_with("export"),
        Lang::Rust => declaration.starts_with("pub"),
        Lang::Go => definition
            .name
            .chars()
            .next()
            .is_some_and(|first| first.is_uppercase()),
        Lang::Python => !definition.name.starts_with('_'),
        Lang::Bash | Lang::Other => false,
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
#[must_use]
pub fn is_exempt(definition: &Definition) -> bool {
    let name = &definition.name;
    name == ENTRY_NAME
        || (name.starts_with("__") && name.ends_with("__"))
        || RUNNER_PREFIXES
            .iter()
            .any(|prefix| name.len() > prefix.len() && name.starts_with(prefix))
}
