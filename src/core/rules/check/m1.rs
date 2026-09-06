//! M1, a test mocks the unit under change.
//!
//! A double is a decision about what a test is for: this part is somebody
//! else's problem today. That decision goes wrong the moment the doubled part
//! is the part being changed, the suite exercises the stand-in, the change
//! ships unexercised, and the run is green because nothing ran.
//!
//! weed only says so where both halves are in front of it. A test file in the
//! diff is read for the doubles it sets up, and each one is resolved against
//! the production files the same diff touches. That happens two ways, because
//! doubles are named two ways: by module, where the framework is handed a path
//! or a dotted module name, and by symbol, where the double wears the name of
//! the thing it stands in for. A double of something outside the diff is a
//! decision weed has no opinion about.

use crate::core::change::Change;
use crate::core::classify::FileKind;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::rules::check::vocab::{doubled_symbol, is_mock, names};
use crate::core::rules::check::Judgement;
use crate::core::syntax::Mask;

/// The extensions a module specifier leaves off, by the language that wrote it.
const SOURCE_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "rs", "go"];

/// The file a directory's module specifier resolves to, per language.
const DIRECTORY_MODULES: &[&str] = &["index", "__init__", "mod"];

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let changes = judged.changes;
    let production: Vec<&Change> = changes
        .iter()
        .filter(|change| change.after.is(FileKind::Prod) || change.before.is(FileKind::Prod))
        .collect();
    if production.is_empty() {
        return Vec::new();
    }

    let mut findings = Vec::new();
    for change in changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !change.after.holds_tests() {
            continue;
        }
        let mask = change.after.mask();
        for (line, double) in doubles(mask) {
            if let Some(covered) = resolve(&double, path, &production) {
                findings.push(finding(path, line, &double, covered));
            }
        }
    }
    findings
}

/// What a double stands in for, however the framework was told.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Double {
    /// A module, named by the path or the dotted name the framework was handed.
    Module(String),
    /// A symbol, worn by the double itself.
    Symbol(String),
}

impl Double {
    fn written(&self) -> &str {
        match self {
            Double::Module(named) | Double::Symbol(named) => named,
        }
    }
}

/// Every double a file sets up, each at the line it was first set up on. The
/// same double reached for in twenty cases is one decision, so it is one
/// finding.
fn doubles(mask: &Mask) -> Vec<(u32, Double)> {
    let mut found: Vec<(u32, Double)> = Vec::new();
    for line in 1..=mask.line_count() {
        let code = mask.code(line);
        for name in names(&code) {
            let Some(doubling) = name.segments.iter().find(|segment| is_mock(segment)) else {
                continue;
            };
            // A double handed a module gets it as the first thing in the call:
            // the specifier is a string, and the framework resolves it the way
            // the language resolves an import. A double that wears the name of
            // what it stands in for needs no argument at all.
            let double = name
                .is_called()
                .then(|| first_literal(mask, line, code[..name.end].chars().count()))
                .flatten()
                .map(Double::Module)
                .or_else(|| doubled_symbol(doubling).map(Double::Symbol));
            if let Some(double) = double {
                if !found.iter().any(|(_, seen)| *seen == double) {
                    found.push((line, double));
                }
            }
        }
    }
    found
}

/// The first string written after the `at`-th character of the line, without
/// its delimiters.
fn first_literal(mask: &Mask, line: u32, at: usize) -> Option<String> {
    let inside = mask
        .literal_runs(line)
        .into_iter()
        .find(|literal| literal.start >= at)?
        .text
        .trim_matches(['"', '\'', '`'])
        .trim()
        .to_string();
    (!inside.is_empty()).then_some(inside)
}

/// The changed production file a double stands in for, where the diff holds one.
fn resolve<'a>(double: &Double, test: &str, production: &[&'a Change]) -> Option<&'a str> {
    production
        .iter()
        .filter_map(|change| Some((*change, change.path()?)))
        .find(|(change, path)| match double {
            Double::Module(module) => names_module(module, test, path),
            Double::Symbol(symbol) => change
                .after
                .outline
                .flatten()
                .iter()
                .any(|definition| definition.name == *symbol),
        })
        .map(|(_, path)| path)
}

/// Whether a module specifier names this production file.
///
/// A specifier written as a relative path is resolved against the directory the
/// test sits in, the way the language resolves an import. A specifier written
/// as a dotted module name is matched against the tail of the path, because the
/// root it counts from is the runner's to know and not weed's, and the tail is
/// tried shorter and shorter, so a specifier that names an attribute inside a
/// module still finds the module.
fn names_module(module: &str, test: &str, path: &str) -> bool {
    let stem = strip_extension(path);
    let mut candidates: Vec<String> = Vec::new();
    if module.starts_with("./") || module.starts_with("../") {
        if let Some(resolved) = relative_to(directory(test), module) {
            candidates.push(resolved);
        }
    } else {
        let dotted = module.replace('.', "/");
        let segments: Vec<&str> = dotted.split('/').filter(|part| !part.is_empty()).collect();
        for length in (1..=segments.len()).rev() {
            candidates.push(segments[..length].join("/"));
        }
    }

    candidates.iter().any(|candidate| {
        stem == *candidate
            || DIRECTORY_MODULES
                .iter()
                .any(|inside| stem == format!("{candidate}/{inside}"))
            || stem
                .strip_suffix(candidate.as_str())
                .is_some_and(|before| before.is_empty() || before.ends_with('/'))
    })
}

/// A relative specifier resolved against the directory it was written in.
fn relative_to(directory: &str, module: &str) -> Option<String> {
    let mut segments: Vec<&str> = directory
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for part in module.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                segments.pop()?;
            }
            other => segments.push(other),
        }
    }
    Some(segments.join("/"))
}

fn directory(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(directory, _)| directory)
}

/// A path without the extension its language writes, and with nothing else
/// taken off: a name carrying dots of its own keeps them.
fn strip_extension(path: &str) -> String {
    match path.rsplit_once('.') {
        Some((stem, extension)) if SOURCE_EXTENSIONS.contains(&extension) => stem.to_string(),
        _ => path.to_string(),
    }
}

fn finding(path: &str, line: u32, double: &Double, covered: &str) -> Finding {
    let what = format!(
        "this test stands a double in for `{}`, which the same change edits in `{covered}`.",
        double.written()
    );
    Finding {
        rule: "M1".to_string(),
        level: Level::Warn,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what,
            why: "the double answers instead of the code that changed, so the case passes without the change ever running.".to_string(),
            next: "let the test reach the real unit, or point the case at what the double is standing in for.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}
