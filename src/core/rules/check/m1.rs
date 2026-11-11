//! M1, a test mocks the unit it should exercise.
//!
//! A double is a decision about what a test is for: this part is somebody
//! else's problem today. That decision goes wrong the moment the doubled part
//! is the part being changed, the suite exercises the stand-in, the change
//! ships unexercised, and the run is green because nothing ran.
//!
//! weeder only says so where both halves are in front of it. A test file in the
//! diff is read for the doubles it sets up, and each one is resolved against
//! the production files the same diff touches. That happens two ways, because
//! doubles are named two ways: by module, where the framework is handed a path
//! or a dotted module name, and by symbol, where the double wears the name of
//! the thing it stands in for.
//!
//! The same decision goes wrong when the doubled part is the part the test is
//! about, whether or not the change edits it: `vi.mock("./format")` at the top
//! of `format.test.ts`, or a `MockFormatter` in `TestFormatterPads`. So a
//! double the change sets up is also read against what the test says it is
//! about: a module double against the file the test file is named for, and
//! any double against the title of the case it serves, an added one, with the
//! suites around it. Only a double on a line the change added is asked this,
//! because a suite that already doubled its subject was judged when it did.
//! A double of anything else is a decision weeder has no opinion about.

use std::collections::BTreeSet;

use crate::core::change::Change;
use crate::core::classify::FileKind;
use crate::core::finding::{Detection, Message, Region, Stamp};
use crate::core::read::{DefinitionKind, TestUnit, TestUnitKind};
use crate::core::rules::check::added::added_cases;
use crate::core::rules::check::vocab::{doubled_symbol, holds_word, is_mock, names, parts};
use crate::core::rules::check::Judgement;
use crate::core::syntax::Mask;

/// The extensions a module specifier leaves off, by the language that wrote it.
const SOURCE_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "rs", "go"];

/// The file a directory's module specifier resolves to, per language.
const DIRECTORY_MODULES: &[&str] = &["index", "__init__", "mod"];

/// What a test file's name carries beside the name of the file it tests, by
/// the conventions its runners collect it under: `format.test.ts`,
/// `format.spec.ts`, `test_format.py`, `format_test.go`.
const TEST_SUFFIXES: &[&str] = &[".test", ".spec", "_test"];
const TEST_PREFIXES: &[&str] = &["test_"];

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let changes = judged.changes;
    let production: Vec<&Change> = changes
        .iter()
        .filter(|change| change.after.is(FileKind::Prod) || change.before.is(FileKind::Prod))
        .collect();

    let mut findings = Vec::new();
    for change in changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !change.after.holds_tests() {
            continue;
        }
        let placed: Vec<(u32, Double)> = set_up(change.after.mask())
            .into_iter()
            .filter(|(line, _)| !names_a_function(change, *line))
            .collect();
        let mut reported: Vec<Double> = Vec::new();
        for (line, double) in first_of_each(&placed) {
            if let Some(covered) = resolve(&double, path, &production) {
                findings.push(finding(path, line, &double, covered));
                reported.push(double);
            }
        }
        let added: BTreeSet<u32> = change.added().map(|(line, _)| line).collect();
        let cases = added_cases(change);
        for (line, double) in placed {
            if !added.contains(&line) || reported.contains(&double) {
                continue;
            }
            if let Some(about) = about(&double, line, path, change, &cases) {
                findings.push(subject_finding(path, line, &double, &about));
                reported.push(double);
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
fn first_of_each(placed: &[(u32, Double)]) -> Vec<(u32, Double)> {
    let mut found: Vec<(u32, Double)> = Vec::new();
    for (line, double) in placed {
        if !found.iter().any(|(_, seen)| seen == double) {
            found.push((*line, double.clone()));
        }
    }
    found
}

/// Every place a file sets a double up, in the order it writes them.
fn set_up(mask: &Mask) -> Vec<(u32, Double)> {
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
                found.push((line, double));
            }
        }
    }
    found
}

/// Whether the line declares a function whose own name carries a word for a
/// double: `fn a_double_answers()`, `def test_spy_records()`, `fn double_of()`.
/// A function is named, not doubled, and a test named for the double it
/// checks carries the same words in its title, so the name is read as a
/// declaration and never as a double set up. A type declared with the word,
/// `struct MockClock`, is a double and is still read as one.
fn names_a_function(change: &Change, line: u32) -> bool {
    change.after.outline.flatten().iter().any(|definition| {
        definition.kind == DefinitionKind::Function
            && definition.start_line == line
            && is_mock(&definition.name)
    })
}

/// Why a test is about what a double stands in for.
enum About {
    /// The double stands in for the file the test file is named for.
    TheFile,
    /// The double stands in for what this case's title, or a suite's around
    /// it, names.
    TheCase(String),
}

/// What makes the test about what the double stands in for, where something
/// does: the file the test file is named for, or the title of the case the
/// double serves. A double set up inside a case serves that case, and one the
/// case did not add to the file is not this change's to answer for; a double
/// set up outside every case serves every case the change added.
fn about(
    double: &Double,
    line: u32,
    test: &str,
    change: &Change,
    cases: &[&TestUnit],
) -> Option<About> {
    let doubled = doubled_name(double);
    if matches!(double, Double::Module(_)) && Some(doubled) == tested_stem(test) {
        return Some(About::TheFile);
    }
    let units = &change.after.tests.units;
    let inside = units
        .iter()
        .filter(|unit| unit.kind == TestUnitKind::Case)
        .filter(|unit| unit.start_line <= line && line <= unit.end_line)
        .min_by_key(|unit| unit.end_line.saturating_sub(unit.start_line));
    let served: Vec<&TestUnit> = match inside {
        Some(case) => cases
            .iter()
            .copied()
            .filter(|added| *added == case)
            .collect(),
        None => cases.to_vec(),
    };
    let words = parts(doubled);
    served
        .into_iter()
        .find(|case| {
            let titles = titles(units, case);
            let named: Vec<&str> = titles.iter().map(String::as_str).collect();
            !words.is_empty() && words.iter().all(|word| holds_word(word, &named))
        })
        .map(|case| About::TheCase(case.name.clone()))
}

/// The words a case's title and the titles of the suites around it are made
/// of, each title read as the words of its names and the words of its prose.
fn titles(units: &[TestUnit], case: &TestUnit) -> Vec<String> {
    units
        .iter()
        .filter(|unit| {
            *unit == case
                || (unit.kind == TestUnitKind::Suite
                    && unit.start_line <= case.start_line
                    && case.end_line <= unit.end_line)
        })
        .flat_map(|unit| {
            unit.name
                .split(|character: char| !character.is_alphanumeric() && character != '_')
                .flat_map(parts)
                .collect::<Vec<String>>()
        })
        .collect()
}

/// The last name a double stands in for: the symbol it wears, or the last
/// segment of the module it was handed, without its extension.
fn doubled_name(double: &Double) -> &str {
    match double {
        Double::Symbol(symbol) => symbol,
        Double::Module(module) => {
            let file = module.rsplit('/').next().unwrap_or(module);
            let file = match file.rsplit_once('.') {
                Some((stem, extension)) if SOURCE_EXTENSIONS.contains(&extension) => stem,
                _ => file,
            };
            file.rsplit('.').next().unwrap_or(file)
        }
    }
}

/// The name of the file a test file is named for, where its name follows a
/// runner's convention: `format` for `format.test.ts` and for
/// `test_format.py`. A Rust suite under `tests/` is named for what it tests
/// without any mark of its own.
fn tested_stem(test: &str) -> Option<&str> {
    let name = test.rsplit('/').next().unwrap_or(test);
    let stem = match name.rsplit_once('.') {
        Some((stem, extension)) if SOURCE_EXTENSIONS.contains(&extension) => stem,
        _ => return None,
    };
    let unmarked = TEST_SUFFIXES
        .iter()
        .find_map(|suffix| stem.strip_suffix(suffix))
        .or_else(|| {
            TEST_PREFIXES
                .iter()
                .find_map(|prefix| stem.strip_prefix(prefix))
        })
        .or_else(|| {
            let integration = name.ends_with(".rs") && test.split('/').any(|part| part == "tests");
            integration.then_some(stem)
        })?;
    (!unmarked.is_empty()).then_some(unmarked)
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
/// root it counts from is the runner's to know and not weeder's, and the tail is
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

fn subject_finding(path: &str, line: u32, double: &Double, about: &About) -> Detection {
    let because = match about {
        About::TheFile => "the file the test is named for".to_string(),
        About::TheCase(case) => format!("what the added case `{case}` is named for"),
    };
    Detection {
        stamp: Stamp::Rule,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: format!(
                "this test stands a double in for `{}`, which is {because}.",
                double.written()
            ),
            why: "the double answers instead of the code the test is about, so the case passes without that code ever running.".to_string(),
            next: "let the test reach the real unit, and double only what the unit calls.".to_string(),
        },
        fix: None,
    }
}

fn finding(path: &str, line: u32, double: &Double, covered: &str) -> Detection {
    let what = format!(
        "this test stands a double in for `{}`, which the same change edits in `{covered}`.",
        double.written()
    );
    Detection {
        stamp: Stamp::Rule,
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
    }
}
