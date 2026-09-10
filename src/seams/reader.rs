//! The reader seam: everything weeder knows about the inside of a file.
//!
//! weeder parses nothing. Outlines, language detection, test structure, imports
//! and call sites all come from `tilth-core`, and this is the only module that
//! names it. What leaves here is the plain data in [`crate::core::read`], so an
//! upgrade of the substrate is a change to this file and to nothing else.
//!
//! Reading is by path and content: the path names the language, the content is
//! what the caller already holds, a file from the working tree, or a blob at a
//! ref that no longer exists on disk. Only [`related_files`] and [`callers`]
//! touch the filesystem, because only they answer questions about other files.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use tilth_core::{
    detect_file_type, extract_import_source, find_callers_batch, get_outline_entries, is_external,
    is_import_line, resolve_related_files_with_content, test_entries, BloomFilterCache, FileType,
    OutlineEntry, OutlineKind, TestKind,
};

use crate::core::classify::Lang;
use crate::core::read::{
    CallerSite, Definition, DefinitionKind, Import, Outline, TestShape, TestUnit, TestUnitKind,
};

/// How many call sites one search collects before it stops walking. A rule that
/// names a blast radius has what it needs long before this; the limit is what
/// keeps a symbol called everywhere from turning a check into a survey.
const CALLER_LIMIT: usize = 200;

/// Reading failed on the way to the answer. The scope travels with the message,
/// so a face can say which tree weeder could not walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    pub scope: String,
    pub message: String,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not read {}: {}.", self.scope, self.message)
    }
}

impl std::error::Error for ReadError {}

/// The language a path is written in, as far as weeder has rules for it. A file
/// in a language weeder carries no rules for, and anything that is not code at
/// all, reads as [`Lang::Other`].
#[must_use]
pub fn language(path: &Path) -> Lang {
    match code_language(path) {
        Some(tilth_core::Lang::TypeScript | tilth_core::Lang::Tsx) => Lang::TypeScript,
        Some(tilth_core::Lang::JavaScript) => Lang::JavaScript,
        Some(tilth_core::Lang::Python) => Lang::Python,
        Some(tilth_core::Lang::Rust) => Lang::Rust,
        Some(tilth_core::Lang::Go) => Lang::Go,
        _ => Lang::Other,
    }
}

/// What a file declares. A file in a language with no grammar behind it, or one
/// that declares nothing, outlines as empty.
#[must_use]
pub fn outline(path: &Path, content: &str) -> Outline {
    let Some(lang) = code_language(path) else {
        return Outline::default();
    };
    let mut definitions: Vec<Definition> = get_outline_entries(content, lang)
        .into_iter()
        .map(definition)
        .collect();
    // Go declares a type with a keyword the grammar behind the outline emits no
    // entry for, and a rule that asks what a file defines has to be told about
    // them: the interface a double stands in for is a definition like any other.
    if lang == tilth_core::Lang::Go {
        definitions.extend(declared_types(content));
        definitions.sort_by_key(|definition| definition.start_line);
    }
    Outline { definitions }
}

/// The tests a file declares.
///
/// Languages that spell a test as a call, `describe`, `it`, `test`, are read
/// from those calls, with the title the call was given. Languages that spell a
/// test as a declaration are read from the outline by their own convention: a
/// Python class named `Test…` holding functions named `test_…`, a Rust function
/// under a `#[test]` attribute inside a `#[cfg(test)]` module, a Go function
/// named `TestX` that takes the handle from `testing`. A file that declares no
/// tests has an empty shape, which is what makes "this file used to hold tests"
/// a thing a rule can see.
#[must_use]
pub fn test_shape(path: &Path, content: &str) -> TestShape {
    let Some(lang) = code_language(path) else {
        return TestShape::default();
    };

    let mut units: Vec<TestUnit> = test_entries(content, lang)
        .into_iter()
        .map(|entry| TestUnit {
            kind: match entry.kind {
                TestKind::Suite => TestUnitKind::Suite,
                TestKind::Case => TestUnitKind::Case,
            },
            name: entry.name,
            start_line: entry.start_line,
            end_line: entry.end_line,
            depth: entry.depth,
        })
        .collect();

    let lines: Vec<&str> = content.lines().collect();
    declared_tests(
        lang,
        &lines,
        &outline(path, content).definitions,
        0,
        &mut units,
    );
    units.sort_by_key(|unit| (unit.start_line, unit.depth));
    TestShape { units }
}

/// The import statements a file makes, in source order.
///
/// tilth judges an import by the line it starts on, and so does weeder: a
/// statement written across several lines carries the range it spans and the
/// source read off the whole of it. A language that lists several sources in
/// one statement, Go's parenthesised block, is one import here, because the
/// substrate names one source per statement and weeder adds no parser of its own.
#[must_use]
pub fn imports(path: &Path, content: &str) -> Vec<Import> {
    let Some(lang) = code_language(path) else {
        return Vec::new();
    };

    // The outline is what knows where a multi-line statement ends; the line
    // scan is what finds every statement, including the ones no outline entry
    // is emitted for.
    let ends = import_statement_ends(path, content);
    let lines: Vec<&str> = content.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !is_import_line(line, lang) {
            continue;
        }
        let start_line = index as u32 + 1;
        let end_line = ends
            .get(&start_line)
            .copied()
            .unwrap_or(start_line)
            .max(start_line)
            .min(lines.len() as u32);
        let text = lines[index..end_line as usize]
            .iter()
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join(" ");
        let source = extract_import_source(&text, Some(lang));
        found.push(Import {
            start_line,
            end_line,
            external: is_external(&source, lang),
            text,
            source,
        });
    }
    found
}

/// The files this file's relative imports resolve to on disk. An import that
/// names a package, or one whose module path needs a build system to resolve,
/// resolves to nothing.
#[must_use]
pub fn related_files(path: &Path, content: &str) -> Vec<PathBuf> {
    resolve_related_files_with_content(path, content)
}

/// A path the reader found, as the repository spells it: relative to the scope
/// it was searched under and joined with `/`, so a finding names `src/app/main.ts`
/// on every platform and never the directory the checkout happens to sit in.
/// A path outside the scope is kept whole, since there is nothing to make it
/// relative to.
fn within(scope: &Path, found: &Path) -> PathBuf {
    let relative = found.strip_prefix(scope).unwrap_or(found);
    let spelled = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<String>>()
        .join("/");
    PathBuf::from(spelled)
}

/// Every call site of `symbols` under `scope`, sorted by file and line so two
/// runs of weeder produce the same findings in the same order.
///
/// The walk honours the ignore files the tree carries, so a build directory is
/// not a blast radius.
pub fn callers(symbols: &BTreeSet<String>, scope: &Path) -> Result<Vec<CallerSite>, ReadError> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }

    let targets: HashSet<String> = symbols.iter().cloned().collect();
    let bloom = BloomFilterCache::new();
    let found =
        find_callers_batch(&targets, scope, &bloom, None, CALLER_LIMIT).map_err(|error| {
            ReadError {
                scope: scope.display().to_string(),
                message: error.to_string(),
            }
        })?;

    let mut sites: Vec<CallerSite> = found
        .into_iter()
        .map(|(symbol, site)| CallerSite {
            symbol,
            path: within(scope, &site.path),
            line: site.line,
            calling_function: site.calling_function,
            call_text: site.call_text.trim().to_string(),
        })
        .collect();
    sites.sort_by(|left, right| {
        (&left.path, left.line, &left.symbol, &left.calling_function).cmp(&(
            &right.path,
            right.line,
            &right.symbol,
            &right.calling_function,
        ))
    });
    Ok(sites)
}

/// The language tilth reads a path as, or `None` where the path is not code.
fn code_language(path: &Path) -> Option<tilth_core::Lang> {
    match detect_file_type(path) {
        FileType::Code(lang) => Some(lang),
        _ => None,
    }
}

fn definition(entry: OutlineEntry) -> Definition {
    Definition {
        kind: definition_kind(entry.kind),
        name: entry.name,
        start_line: entry.start_line,
        end_line: entry.end_line,
        signature: entry.signature,
        children: entry.children.into_iter().map(definition).collect(),
    }
}

/// tilth's word for a declaration, in weeder's own vocabulary. A binding that
/// cannot be reassigned is a constant here whichever keyword declared it, and
/// the two kinds no grammar emits arrive as `Other` rather than as a promise.
fn definition_kind(kind: OutlineKind) -> DefinitionKind {
    match kind {
        OutlineKind::Import => DefinitionKind::Import,
        OutlineKind::Function => DefinitionKind::Function,
        OutlineKind::Class => DefinitionKind::Class,
        OutlineKind::Struct => DefinitionKind::Struct,
        OutlineKind::Interface => DefinitionKind::Interface,
        OutlineKind::TypeAlias => DefinitionKind::TypeAlias,
        OutlineKind::Enum => DefinitionKind::Enum,
        OutlineKind::Constant | OutlineKind::ImmutableVariable => DefinitionKind::Constant,
        OutlineKind::Variable => DefinitionKind::Variable,
        OutlineKind::Export => DefinitionKind::Export,
        OutlineKind::Property => DefinitionKind::Property,
        OutlineKind::Module => DefinitionKind::Module,
        OutlineKind::TestSuite | OutlineKind::TestCase => DefinitionKind::Other,
    }
}

/// Where each import statement in the outline ends, keyed by the line it starts
/// on. Only imports, so a declaration that happens to open on the same line as
/// one cannot lend it its range.
fn import_statement_ends(path: &Path, content: &str) -> HashMap<u32, u32> {
    outline(path, content)
        .flatten()
        .iter()
        .filter(|definition| definition.kind == DefinitionKind::Import)
        .map(|definition| (definition.start_line, definition.end_line))
        .collect()
}

/// Walk the outline for tests a language declares rather than calls, giving
/// each one the depth of the suites it sits inside.
fn declared_tests(
    lang: tilth_core::Lang,
    lines: &[&str],
    definitions: &[Definition],
    depth: u8,
    into: &mut Vec<TestUnit>,
) {
    for definition in definitions {
        let kind = declared_test_kind(lang, lines, definition);
        if let Some(kind) = kind {
            into.push(TestUnit {
                kind,
                name: definition.name.clone(),
                start_line: definition.start_line,
                end_line: definition.end_line,
                depth,
            });
        }
        let inside = if kind.is_some() {
            depth.saturating_add(1)
        } else {
            depth
        };
        declared_tests(lang, lines, &definition.children, inside, into);
    }
}

/// Whether a declaration is a test, by the convention of the language that
/// declares it. Languages that spell tests as calls declare none, and answer
/// `None` here.
fn declared_test_kind(
    lang: tilth_core::Lang,
    lines: &[&str],
    definition: &Definition,
) -> Option<TestUnitKind> {
    match lang {
        // pytest and unittest: a class named `Test…` groups methods named
        // `test_…`, and a bare `test_…` function is a test on its own.
        tilth_core::Lang::Python => match definition.kind {
            DefinitionKind::Class if definition.name.starts_with("Test") => {
                Some(TestUnitKind::Suite)
            }
            DefinitionKind::Function
                if definition.name == "test" || definition.name.starts_with("test_") =>
            {
                Some(TestUnitKind::Case)
            }
            _ => None,
        },
        // Rust marks a test with an attribute and the module that holds them
        // with `#[cfg(test)]`; neither is visible in the outline, so the lines
        // above the declaration are where they are read.
        tilth_core::Lang::Rust => {
            let attributes = attributes_above(lines, definition.start_line);
            match definition.kind {
                DefinitionKind::Function
                    if attributes
                        .iter()
                        .any(|attribute| is_test_attribute(attribute)) =>
                {
                    Some(TestUnitKind::Case)
                }
                DefinitionKind::Module
                    if attributes
                        .iter()
                        .any(|attribute| is_cfg_test_attribute(attribute)) =>
                {
                    Some(TestUnitKind::Suite)
                }
                _ => None,
            }
        }
        // `go test` runs a function whose name carries the kind of test it is
        // and whose parameter is the handle that runs it.
        tilth_core::Lang::Go => {
            let named_for_testing = ["Test", "Benchmark", "Fuzz"].iter().any(|prefix| {
                definition.name.len() > prefix.len() && definition.name.starts_with(prefix)
            });
            let takes_the_handle = definition
                .signature
                .as_deref()
                .is_some_and(|signature| signature.contains("*testing."));
            (definition.kind == DefinitionKind::Function && named_for_testing && takes_the_handle)
                .then_some(TestUnitKind::Case)
        }
        _ => None,
    }
}

/// The attributes written immediately above a declaration, without their `#[`
/// and `]`. Doc comments may sit among them and are walked past.
fn attributes_above(lines: &[&str], start_line: u32) -> Vec<String> {
    let mut found = Vec::new();
    let mut index = start_line as usize;
    while index > 1 {
        index -= 1;
        let line = lines.get(index - 1).map_or("", |line| line.trim());
        if let Some(attribute) = line
            .strip_prefix("#[")
            .and_then(|rest| rest.strip_suffix(']'))
        {
            found.push(attribute.to_string());
        } else if !line.starts_with("//") {
            break;
        }
    }
    found
}

/// Whether an attribute is the one that makes a function a test: `#[test]`, and
/// the runtimes that wrap it (`#[tokio::test]`).
fn is_test_attribute(attribute: &str) -> bool {
    let path = attribute.split('(').next().unwrap_or(attribute).trim();
    path.rsplit("::").next().unwrap_or(path) == "test"
}

/// Whether an attribute is `#[cfg(test)]`: the mark on a module that is only
/// compiled when the tests are.
fn is_cfg_test_attribute(attribute: &str) -> bool {
    attribute
        .trim()
        .strip_prefix("cfg(")
        .and_then(|rest| rest.strip_suffix(')'))
        .is_some_and(|condition| {
            condition
                .split(|character: char| !character.is_alphanumeric() && character != '_')
                .any(|token| token == "test")
        })
}

/// The types a Go file declares, written one at a time or gathered in a
/// parenthesised block. Go declares its types at the top of the file and
/// nowhere else, so the scan reads the lines that begin one and follows the
/// braces to the end of it.
fn declared_types(content: &str) -> Vec<Definition> {
    let lines: Vec<&str> = content.lines().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let Some(rest) = lines[index].strip_prefix("type ") else {
            index += 1;
            continue;
        };
        if rest.trim() == "(" {
            index += 1;
            while index < lines.len() && lines[index].trim() != ")" {
                if let Some(declared) = declared_type(&lines, index) {
                    index = declared.end_line as usize;
                    found.push(declared);
                }
                index += 1;
            }
            index += 1;
            continue;
        }
        if let Some(declared) = declared_type(&lines, index) {
            index = declared.end_line as usize;
            found.push(declared);
        }
        index += 1;
    }
    found
}

/// One type declaration: the name it gives, what it is made of, and the lines it
/// occupies. A declaration that opens a brace runs to the brace that closes it.
fn declared_type(lines: &[&str], index: usize) -> Option<Definition> {
    let line = lines.get(index)?;
    let declaration = line.trim().strip_prefix("type ").unwrap_or(line.trim());
    let mut words = declaration.split_whitespace();
    let name = words.next()?;
    if !name
        .chars()
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
    {
        return None;
    }
    let kind = match words.next() {
        Some(word) if word.starts_with("interface") => DefinitionKind::Interface,
        Some(word) if word.starts_with("struct") => DefinitionKind::Struct,
        Some(_) => DefinitionKind::TypeAlias,
        None => return None,
    };
    Some(Definition {
        kind,
        name: name.trim_end_matches(&['[', ','][..]).to_string(),
        start_line: index as u32 + 1,
        end_line: block_end(lines, index) as u32 + 1,
        signature: Some(line.trim().to_string()),
        children: Vec::new(),
    })
}

/// Where the block a line opens is closed, or the line itself where it opens
/// none. A file that ends inside a block ends there, and so does the count.
fn block_end(lines: &[&str], index: usize) -> usize {
    let mut depth: i32 = 0;
    for (offset, line) in lines.iter().enumerate().skip(index) {
        for character in line.chars() {
            match character {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
        if depth <= 0 {
            return offset;
        }
    }
    lines.len().saturating_sub(1)
}
