//! What a runner collects, and what the configuration around it keeps out.
//!
//! T7 reads the name a file was renamed to, because every runner decides what
//! to run by name. This reads the other half of that decision: the settings a
//! repository writes to say which of those files are run at all. An ignore
//! list, a path pattern narrowed to one directory, a gate at the top of a
//! suite, each takes a file out of the run while leaving it in the tree, and a
//! reader of the diff sees the file and not the run.
//!
//! Two rules ask the question and one collector answers it, so `check` judging
//! a line and `scan` judging a tree can never disagree about what the runner
//! will open. What the collector hands back is a set of suites, each with the
//! file and line that keeps it out.
//!
//! Nothing here is named after a product. A settings file is found by the
//! classifier's kind and by the name its ecosystem gives it, the way the
//! classifier already finds a manifest; what a setting means is read from the
//! words its key is spelled with, `ignore` and `exclude` on one side, `path`,
//! `files` and `match` on the other; and a setting counts as being about the
//! run only where the name it is written under says so, the key, the table or
//! object it sits in, or the file itself. A repository whose runner reads a
//! settings file weeder cannot tell from any other is a repository weeder says
//! nothing about, which is the honest half of the split: the alternative is a
//! list of tools, and a list of tools rots.
//!
//! Two shapes are left to the reader rather than guessed at. A value built by
//! code, rather than written down, is reported unreadable instead of being read
//! wrongly. And a test target gated on a feature nobody enables is a question
//! about how the crate is built rather than about what the manifest says, so
//! `required-features` is not read here.

use std::collections::BTreeSet;

use crate::core::change::Change;
use crate::core::classify::{classify_file, FileKind, Lang};
use crate::core::glob;
use crate::core::syntax::Mask;

/// One file the collector reads: what it is called, what the classifier made of
/// it, and the one scan of its text the face already did.
#[derive(Debug, Clone, Copy)]
pub struct File<'a> {
    pub path: &'a str,
    pub kind: FileKind,
    pub lang: Lang,
    pub mask: &'a Mask,
}

/// A test file the configuration keeps out of the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hidden {
    /// The suite the runner will not collect.
    pub test: String,
    /// The file whose line keeps it out.
    pub path: String,
    /// The 1-based line of that file.
    pub line: u32,
    /// The setting, as that file spells it.
    pub setting: String,
    /// Whether the line names what it hides. A line that narrows a pattern
    /// leaves the hidden set to be worked out from everything else the tree
    /// holds, and weeder reports that as something to read rather than as
    /// something to stop.
    pub named: bool,
}

/// A setting weeder cannot read, because its value is built rather than
/// written. What such a line keeps out of the run is not in the file, and a
/// guess would be worse than saying so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    pub path: String,
    pub line: u32,
    pub setting: String,
}

impl Unreadable {
    /// The one line weeder says about a setting it could not read.
    #[must_use]
    pub fn complaint(&self) -> String {
        format!(
            "{}:{} states {} as something weeder cannot read: the value is built rather than written, so what it keeps out of the run is not in the file.",
            self.path, self.line, self.setting
        )
    }
}

/// What the configuration says about what runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Collection {
    pub hidden: Vec<Hidden>,
    pub unreadable: Vec<Unreadable>,
}

impl Hidden {
    /// Where the suite was taken out of the run, spelled so a reader can go
    /// straight there. A gate is written inside the suite it hides, and naming
    /// the same file twice reads as a mistake rather than as a place.
    #[must_use]
    pub fn at(&self) -> String {
        if self.test == self.path {
            format!("the {} at {}:{}", self.setting, self.path, self.line)
        } else {
            format!("{}:{} ({})", self.path, self.line, self.setting)
        }
    }
}

/// One test file of the tree, and the language whose runner collects it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suite {
    pub path: String,
    pub lang: Lang,
}

/// The directories that hold what a test reads rather than tests. Every runner
/// is told to stay out of them, so a setting that names one takes no suite out
/// of the run.
const MATERIAL: &[&str] = &["fixtures", "testdata"];

/// The words a key is spelled with when the setting takes something out of the
/// run.
const EXCLUDING: &[&str] = &["ignore", "exclude", "deselect", "skip", "omit", "norecurs"];

/// The words a key is spelled with when the setting names what the run
/// collects. Everything the pattern does not reach is out of the run, which is
/// a hidden set nobody wrote down.
const COLLECTING: &[&str] = &[
    "path", "dir", "root", "file", "include", "match", "spec", "glob",
];

/// The words a name is spelled with when what it holds is a command line rather
/// than a list of paths. A value weeder read as arguments without being told it
/// was arguments would make a sentence somebody wrote into a pattern.
const COMMANDED: &[&str] = &["opt", "arg", "command", "cmd", "script", "run"];

/// Every test file the configuration keeps out of the run, with the file and
/// line that keeps each one out.
///
/// `read` is what the face could read the text of, the changed files for
/// `check` and the whole tree for `scan`; `paths` is every path the repository
/// holds, which is what a pattern is judged against.
#[must_use]
pub fn uncollected(read: &[File<'_>], paths: &[String]) -> Collection {
    let suites = suites(paths);
    let mut collection = Collection::default();
    for file in read {
        gated(file, &suites, &mut collection);
        let Some(settings) = settings(file) else {
            continue;
        };
        let stated = match settings.syntax {
            Syntax::Ini => stated_as_ini(file),
            _ => stated_as_tokens(file, settings.syntax),
        };
        interpret(file, &settings, &stated, &suites, &mut collection);
    }
    collection
}

/// One finding per hidden file: where two settings keep the same suite out, the
/// one that names it is the one worth reporting, and the first line that says
/// so is where the reader is sent.
#[must_use]
pub fn one_per_test(hidden: &[Hidden]) -> Vec<Hidden> {
    let mut kept: Vec<Hidden> = Vec::new();
    for entry in hidden {
        match kept.iter_mut().find(|held| held.test == entry.test) {
            Some(held) if entry.named && !held.named => *held = entry.clone(),
            Some(_) => {}
            None => kept.push(entry.clone()),
        }
    }
    kept
}

/// Every test file the tree holds that a runner collects by name.
#[must_use]
pub fn suites(paths: &[String]) -> Vec<Suite> {
    paths
        .iter()
        .filter(|path| !is_material(path))
        .filter_map(|path| {
            let classification = classify_file(path, "");
            (classification.kind == FileKind::Test && collects_file(classification.lang, path))
                .then(|| Suite {
                    path: path.clone(),
                    lang: classification.lang,
                })
        })
        .collect()
}

/// Whether the runner collects a file at this path as a suite of its own.
#[must_use]
pub fn collects_file(lang: Lang, path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    let Some((name, directories)) = segments.split_last() else {
        return false;
    };
    match lang {
        // The runners glob for a file whose name carries the word in front of
        // its extension, or for anything inside the directory kept for tests.
        Lang::TypeScript | Lang::JavaScript => {
            name.contains(".test.") || name.contains(".spec.") || directories.contains(&"__tests__")
        }
        Lang::Python => {
            name.starts_with("test_")
                || name
                    .strip_suffix(".py")
                    .is_some_and(|stem| stem.ends_with("_test"))
        }
        Lang::Go => name.ends_with("_test.go"),
        // An integration target is a file at the top of the tests directory,
        // and nowhere below it: a file one directory down is a module nobody
        // compiles until something declares it.
        Lang::Rust => {
            directories == ["tests"]
                || directories == ["benches"]
                || directories.first() == Some(&"src")
        }
        Lang::Other => false,
    }
}

/// Whether a path lies in a directory of material a test reads.
fn is_material(path: &str) -> bool {
    path.split('/')
        .any(|segment| MATERIAL.contains(&segment.to_ascii_lowercase().as_str()))
}

// ---------------------------------------------------------------------------
// The gate a suite carries at the top of itself
// ---------------------------------------------------------------------------

/// A suite the file opens with a condition its own toolchain does not meet.
///
/// A build gate is configuration written inside the file it governs: the suite
/// is compiled, and run, only where the condition holds, and an ordinary run
/// sets none of the words a repository invents for itself.
fn gated(file: &File<'_>, suites: &[Suite], into: &mut Collection) {
    let Some(suite) = suites.iter().find(|suite| suite.path == file.path) else {
        return;
    };
    let Some((line, setting, condition)) = gate(file, suite.lang) else {
        return;
    };
    if satisfied(&condition, suite.lang) {
        return;
    }
    into.hidden.push(Hidden {
        test: suite.path.clone(),
        path: file.path.to_string(),
        line,
        setting: setting.to_string(),
        named: true,
    });
}

/// How far into a file a gate may be written before it is something else. A
/// gate stands above the code, under the licence and the notes.
const GATE_DEPTH: u32 = 20;

/// The gate a file opens with: the line, what the language calls it, and the
/// condition itself.
fn gate<'a>(file: &File<'a>, lang: Lang) -> Option<(u32, &'static str, String)> {
    let last = file.mask.line_count().min(GATE_DEPTH);
    for line in 1..=last {
        match lang {
            // An inner attribute governs everything below it, and the file
            // stops accepting one at its first item.
            Lang::Rust => {
                let code = file.mask.outside_comments(line);
                let code = code.trim();
                if code.is_empty() {
                    continue;
                }
                let attribute = code.strip_prefix("#![")?;
                if let Some(condition) = inside(attribute, "cfg") {
                    return Some((line, "cfg", condition));
                }
            }
            // A build constraint is a comment, and it is one only above the
            // clause that opens the file.
            Lang::Go => {
                if file
                    .mask
                    .outside_comments(line)
                    .trim_start()
                    .starts_with("package ")
                {
                    return None;
                }
                let comment = file.mask.comments(line);
                if let Some(condition) = comment.trim_start().strip_prefix("//go:build ") {
                    return Some((line, "go:build", condition.trim().to_string()));
                }
            }
            _ => return None,
        }
    }
    None
}

/// What a call carries between its brackets, where the text opens with that
/// call. The condition of a gate is written that way in every language that has
/// one.
fn inside(text: &str, called: &str) -> Option<String> {
    let rest = text.trim_start().strip_prefix(called)?.trim_start();
    let rest = rest.strip_prefix('(')?;
    let mut depth = 1;
    let mut held = String::new();
    for character in rest.chars() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(held);
                }
            }
            _ => {}
        }
        held.push(character);
    }
    None
}

/// The words a rust toolchain sets by itself: the platform it builds for, the
/// profile it builds in, and the run being a test run. A condition written from
/// anything else is one somebody has to pass in.
const RUST_SET: &[&str] = &[
    "test",
    "unix",
    "windows",
    "target_os",
    "target_arch",
    "target_family",
    "target_env",
    "target_endian",
    "target_pointer_width",
    "target_vendor",
    "debug_assertions",
    "panic",
    "doc",
    "doctest",
    "miri",
];

/// The words a go toolchain sets by itself: the platforms it builds for, the
/// compilers, and the release it is. A term outside them is a tag somebody has
/// to pass in, and an ordinary run passes none.
const GO_SET: &[&str] = &[
    "aix",
    "android",
    "darwin",
    "dragonfly",
    "freebsd",
    "hurd",
    "illumos",
    "ios",
    "js",
    "linux",
    "nacl",
    "netbsd",
    "openbsd",
    "plan9",
    "solaris",
    "wasip1",
    "windows",
    "zos",
    "386",
    "amd64",
    "amd64p32",
    "arm",
    "armbe",
    "arm64",
    "arm64be",
    "loong64",
    "mips",
    "mipsle",
    "mips64",
    "mips64le",
    "mips64p32",
    "mips64p32le",
    "ppc",
    "ppc64",
    "ppc64le",
    "riscv",
    "riscv64",
    "s390",
    "s390x",
    "sparc",
    "sparc64",
    "wasm",
    "unix",
    "cgo",
    "gc",
    "gccgo",
    "race",
    "msan",
    "asan",
    "boringcrypto",
];

/// Whether an ordinary run meets the condition a file is gated on.
fn satisfied(condition: &str, lang: Lang) -> bool {
    match lang {
        // A negation is a condition weeder will not claim to have followed, so
        // one is read as met rather than as a suite nobody runs.
        Lang::Rust => {
            let words = words_of(condition);
            words.iter().any(|word| word == "not")
                || words.iter().any(|word| RUST_SET.contains(&word.as_str()))
        }
        Lang::Go => constraint(condition).unwrap_or(true),
        _ => true,
    }
}

/// The bare words of a condition, without what its strings hold: a feature is
/// named in a string, and that name is the repository's own rather than one the
/// toolchain knows.
fn words_of(condition: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut quoted = false;
    for run in condition.split('"') {
        if !quoted {
            words.extend(
                run.split(|character: char| !(character.is_alphanumeric() || character == '_'))
                    .filter(|word| !word.is_empty())
                    .map(ToString::to_string),
            );
        }
        quoted = !quoted;
    }
    words
}

/// Whether a build constraint holds for a run that passes no tags of its own:
/// every word the toolchain sets is taken as met and every other word as unset.
/// `None` where the expression is not one weeder can read.
fn constraint(text: &str) -> Option<bool> {
    let mut tokens = text.split_whitespace().flat_map(split_operators).peekable();
    let value = disjunction(&mut tokens)?;
    tokens.next().is_none().then_some(value)
}

/// One word of a constraint as its own tokens, so `!a&&(b)` reads the same as
/// `! a && ( b )`.
fn split_operators(word: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut held = String::new();
    let mut characters = word.chars().peekable();
    while let Some(character) = characters.next() {
        let operator = match character {
            '(' | ')' | '!' => Some(character.to_string()),
            '&' if characters.peek() == Some(&'&') => {
                characters.next();
                Some("&&".to_string())
            }
            '|' if characters.peek() == Some(&'|') => {
                characters.next();
                Some("||".to_string())
            }
            _ => None,
        };
        match operator {
            Some(operator) => {
                if !held.is_empty() {
                    found.push(std::mem::take(&mut held));
                }
                found.push(operator);
            }
            None => held.push(character),
        }
    }
    if !held.is_empty() {
        found.push(held);
    }
    found
}

type Words<I> = std::iter::Peekable<I>;

fn disjunction<I: Iterator<Item = String>>(words: &mut Words<I>) -> Option<bool> {
    let mut value = conjunction(words)?;
    while words.peek().is_some_and(|word| word == "||") {
        words.next();
        value = conjunction(words)? || value;
    }
    Some(value)
}

fn conjunction<I: Iterator<Item = String>>(words: &mut Words<I>) -> Option<bool> {
    let mut value = term(words)?;
    while words.peek().is_some_and(|word| word == "&&") {
        words.next();
        value = term(words)? && value;
    }
    Some(value)
}

fn term<I: Iterator<Item = String>>(words: &mut Words<I>) -> Option<bool> {
    match words.next()?.as_str() {
        "!" => Some(!term(words)?),
        "(" => {
            let value = disjunction(words)?;
            (words.next()? == ")").then_some(value)
        }
        ")" | "&&" | "||" => None,
        tag => Some(GO_SET.contains(&tag) || tag.starts_with("go1.")),
    }
}

// ---------------------------------------------------------------------------
// The settings a file states
// ---------------------------------------------------------------------------

/// How a settings file writes its keys, and whose runner reads them.
struct Settings {
    syntax: Syntax,
    /// The languages whose suites this file decides about.
    governs: &'static [Lang],
}

/// The shapes a settings file is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Syntax {
    /// `key = value` under bracketed headers, values written as data.
    Table,
    /// `key = value` under bracketed headers, values written as they stand and
    /// carried on over indented lines.
    Ini,
    /// Keys nested inside braces.
    Braces,
    /// Statements at the top of a file of code.
    Flat,
}

const PYTHON: &[Lang] = &[Lang::Python];
const RUST: &[Lang] = &[Lang::Rust];
const SCRIPT: &[Lang] = &[Lang::TypeScript, Lang::JavaScript];

/// Whether this file states how the tests are run: the classifier says it is
/// not something a generator wrote, and the name says whose settings they are.
fn settings(file: &File<'_>) -> Option<Settings> {
    if file.kind == FileKind::Generated {
        return None;
    }
    let name = file.path.rsplit('/').next().unwrap_or(file.path);
    let settings = |syntax, governs| Some(Settings { syntax, governs });
    match name {
        "conftest.py" => settings(Syntax::Flat, PYTHON),
        "pyproject.toml" => settings(Syntax::Table, PYTHON),
        "pytest.ini" | "tox.ini" | "setup.cfg" => settings(Syntax::Ini, PYTHON),
        "Cargo.toml" => settings(Syntax::Table, RUST),
        "package.json" => settings(Syntax::Braces, SCRIPT),
        _ => dedicated(file, name),
    }
}

/// A file whose whole purpose is to be read by a tool: the ecosystem spells one
/// `<something>.config.<extension>` or opens its name with a dot and closes the
/// stem with `rc`. A file named for what it holds rather than for who reads it
/// is not one.
fn dedicated(file: &File<'_>, name: &str) -> Option<Settings> {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    if !(stem.ends_with(".config") || (name.starts_with('.') && stem.ends_with("rc"))) {
        return None;
    }
    match file.lang {
        Lang::TypeScript | Lang::JavaScript => Some(Settings {
            syntax: Syntax::Braces,
            governs: SCRIPT,
        }),
        Lang::Other if name.ends_with(".json") => Some(Settings {
            syntax: Syntax::Braces,
            governs: SCRIPT,
        }),
        _ => None,
    }
}

/// One setting a file states: the key, the names it is written under, the line
/// it is written on, and what it says.
#[derive(Debug, Clone, Default)]
struct Stated {
    key: String,
    scope: Vec<String>,
    line: u32,
    entries: Vec<Entry>,
    /// A value written as code rather than as data: weeder can see that the
    /// list is built and not what it will hold.
    built: bool,
    truth: Option<bool>,
    /// Whether the innermost name it sits under is a repeated table, which
    /// declares a target rather than choosing between them.
    declares: bool,
}

/// One value a setting holds, and the line it is written on. A list spread over
/// several lines names each of its entries somewhere, and that is where a
/// reader is sent.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    text: String,
    line: u32,
}

/// One token of a settings line.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// A bare word: a key, a truth value, a name a value is built from.
    Word(String),
    /// A quoted string, without its quotes.
    Text(String),
    /// Anything else, which is what says where a value begins and ends.
    Punct(char),
}

/// The tokens of one line. The comments are gone before this is called, except
/// in the formats weeder's scan has no dialect for, whose marks are handed in
/// here so that a `#` outside a string still ends the line.
fn tokens(line: &str, marks: &[char]) -> Vec<Token> {
    let mut found = Vec::new();
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if character.is_whitespace() {
            continue;
        }
        if marks.contains(&character) {
            break;
        }
        if matches!(character, '"' | '\'' | '`') {
            let mut text = String::new();
            let mut escaped = false;
            for inside in characters.by_ref() {
                if escaped {
                    text.push(inside);
                    escaped = false;
                } else if inside == '\\' {
                    escaped = true;
                } else if inside == character {
                    break;
                } else {
                    text.push(inside);
                }
            }
            found.push(Token::Text(text));
            continue;
        }
        if is_word(character) {
            let mut word = String::from(character);
            while characters.peek().copied().is_some_and(is_word) {
                word.extend(characters.next());
            }
            found.push(Token::Word(word));
            continue;
        }
        found.push(Token::Punct(character));
    }
    found
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-' | '.' | '$')
}

/// The marks that open a comment in a format weeder's scan has no dialect for.
fn marks(lang: Lang) -> &'static [char] {
    match lang {
        Lang::Other => &['#', ';'],
        _ => &[],
    }
}

/// Every setting a file written in brackets and braces states.
fn stated_as_tokens(file: &File<'_>, syntax: Syntax) -> Vec<Stated> {
    let mut found: Vec<Stated> = Vec::new();
    let mut scope: Vec<String> = Vec::new();
    let mut declares = false;
    let mut open: Option<usize> = None;
    let mut depth: i32 = 0;
    for line in 1..=file.mask.line_count() {
        let code = file.mask.outside_comments(line);
        let read = tokens(&code, marks(file.lang));
        if syntax == Syntax::Table {
            if let Some((header, repeated)) = table_header(&read) {
                scope = header;
                declares = repeated;
                open = None;
                continue;
            }
        }
        let mut at = 0;
        while at < read.len() {
            if let Some(index) = open {
                match &read[at] {
                    Token::Text(text) => found[index].entries.push(Entry {
                        text: text.clone(),
                        line,
                    }),
                    Token::Word(word) if !is_number(word) => found[index].built = true,
                    Token::Word(_) => {}
                    Token::Punct(punct) => {
                        depth += nesting(*punct);
                        if depth <= 0 {
                            open = None;
                        }
                    }
                }
                at += 1;
                continue;
            }
            let named = match &read[at] {
                Token::Word(word) | Token::Text(word) => Some(word.clone()),
                Token::Punct('{') => {
                    scope.push(String::new());
                    None
                }
                Token::Punct('}') => {
                    scope.pop();
                    None
                }
                Token::Punct(_) => None,
            };
            let Some(key) =
                named.filter(|_| matches!(read.get(at + 1), Some(Token::Punct(':' | '='))))
            else {
                at += 1;
                continue;
            };
            at += 2;
            match read.get(at) {
                Some(Token::Punct('{')) => {
                    scope.push(key);
                    at += 1;
                }
                Some(Token::Punct('[')) => {
                    found.push(opened(key, &scope, line, declares));
                    open = Some(found.len() - 1);
                    depth = 1;
                    at += 1;
                }
                Some(Token::Text(text)) => {
                    let mut stated = opened(key, &scope, line, declares);
                    stated.entries.push(Entry {
                        text: text.clone(),
                        line,
                    });
                    found.push(stated);
                    at += 1;
                }
                Some(Token::Word(word)) => {
                    let mut stated = opened(key, &scope, line, declares);
                    stated.truth = truth(word);
                    stated.built = stated.truth.is_none() && !is_number(word);
                    found.push(stated);
                    at += 1;
                }
                _ => {}
            }
        }
    }
    found
}

fn opened(key: String, scope: &[String], line: u32, declares: bool) -> Stated {
    Stated {
        key,
        scope: scope.to_vec(),
        line,
        declares,
        ..Stated::default()
    }
}

/// The header a line opens a table with, and whether the table is a repeated
/// one, which declares a target rather than stating settings for all of them.
fn table_header(read: &[Token]) -> Option<(Vec<String>, bool)> {
    if read.first() != Some(&Token::Punct('[')) {
        return None;
    }
    let repeated = read.get(1) == Some(&Token::Punct('['));
    let name = read.get(usize::from(repeated) + 1)?;
    match name {
        Token::Word(word) | Token::Text(word) => Some((vec![word.clone()], repeated)),
        Token::Punct(_) => None,
    }
}

fn nesting(punct: char) -> i32 {
    match punct {
        '[' | '{' => 1,
        ']' | '}' => -1,
        _ => 0,
    }
}

fn truth(word: &str) -> Option<bool> {
    match word {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn is_number(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
}

/// Every setting a file of sections and raw values states. A value here is text
/// as it stands, with no quotes to take off, and it carries on over the lines
/// indented under it.
fn stated_as_ini(file: &File<'_>) -> Vec<Stated> {
    let mut found: Vec<Stated> = Vec::new();
    let mut scope: Vec<String> = Vec::new();
    let mut open: Option<usize> = None;
    for line in 1..=file.mask.line_count() {
        let raw = file.mask.outside_comments(line);
        let stripped = raw.split(['#', ';']).next().unwrap_or_default();
        let text = stripped.trim();
        if text.is_empty() {
            open = None;
            continue;
        }
        if stripped.starts_with(char::is_whitespace) {
            if let Some(index) = open {
                found[index].entries.push(Entry {
                    text: text.to_string(),
                    line,
                });
            }
            continue;
        }
        open = None;
        if let Some(header) = text
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            scope = vec![header.to_string()];
            continue;
        }
        let Some((key, value)) = text.split_once('=') else {
            continue;
        };
        let mut stated = opened(key.trim().to_string(), &scope, line, false);
        let value = value.trim();
        stated.truth = truth(value);
        if !value.is_empty() && stated.truth.is_none() {
            stated.entries.push(Entry {
                text: value.to_string(),
                line,
            });
        }
        found.push(stated);
        open = Some(found.len() - 1);
    }
    found
}

// ---------------------------------------------------------------------------
// What a setting says about the run
// ---------------------------------------------------------------------------

/// What a setting does to the run.
enum Sense {
    /// It names what is taken out.
    Excluding,
    /// It names what is collected, so everything else is out.
    Collecting,
    /// It turns the finding of suites on or off.
    Switch(bool),
    /// It is a command line, and what the run collects is in the arguments.
    Command,
    /// It says nothing about what runs.
    Quiet,
}

fn sense(stated: &Stated) -> Sense {
    if let Some(truth) = stated.truth {
        return if mentions_tests(&stated.key) {
            Sense::Switch(truth)
        } else {
            Sense::Quiet
        };
    }
    let key = stated.key.to_ascii_lowercase();
    if EXCLUDING.iter().any(|word| key.contains(word)) {
        return Sense::Excluding;
    }
    // A value written as an expression is one weeder matches nothing against.
    if key.contains("regex") {
        return Sense::Quiet;
    }
    if COLLECTING.iter().any(|word| key.contains(word)) {
        return Sense::Collecting;
    }
    let commanded = COMMANDED.iter().any(|word| key.contains(word))
        || stated.scope.iter().any(|held| {
            COMMANDED
                .iter()
                .any(|word| held.to_ascii_lowercase().contains(word))
        });
    if commanded {
        Sense::Command
    } else {
        Sense::Quiet
    }
}

/// Whether a command line is the run, rather than one of the runs. A file that
/// lists runs by name has one the ecosystem starts by itself, and the rest are
/// asked for by name; a narrower one beside the default takes nothing out of
/// the run, because the default still collects it.
fn the_default_run(stated: &Stated) -> bool {
    let listed = stated.scope.iter().any(|held| {
        let held = held.to_ascii_lowercase();
        held.contains("script") || held.contains("command")
    });
    !listed || stated.key.eq_ignore_ascii_case("test")
}

fn mentions_tests(name: &str) -> bool {
    name.to_ascii_lowercase().contains("test")
}

/// Whether the setting says for itself that it is about the tests, rather than
/// being taken for one because of the file it sits in.
fn names_the_tests(stated: &Stated) -> bool {
    mentions_tests(&stated.key) || stated.scope.iter().any(|held| mentions_tests(held))
}

/// Whether a setting is about the run at all: the name it is written under says
/// so, the key, the table or object it sits in, or the file itself.
fn about_tests(file: &File<'_>, stated: &Stated) -> bool {
    let name = file.path.rsplit('/').next().unwrap_or(file.path);
    names_the_tests(stated) || mentions_tests(name)
}

fn interpret(
    file: &File<'_>,
    settings: &Settings,
    stated: &[Stated],
    suites: &[Suite],
    into: &mut Collection,
) {
    let directory = file
        .path
        .rsplit_once('/')
        .map_or("", |(directory, _)| directory);
    let mine: Vec<&Suite> = suites
        .iter()
        .filter(|suite| settings.governs.contains(&suite.lang))
        .collect();
    if mine.is_empty() {
        return;
    }
    for setting in stated {
        if !about_tests(file, setting) {
            continue;
        }
        match sense(setting) {
            Sense::Excluding => {
                if unreadable(file, setting, into) {
                    continue;
                }
                for entry in &setting.entries {
                    for pattern in patterns(&entry.text) {
                        exclude(file, setting, entry.line, &pattern, directory, &mine, into);
                    }
                }
            }
            Sense::Collecting => {
                // A pattern built rather than written narrows nothing weeder
                // can name, so weeder claims nothing about it. Only an
                // exclusion it cannot read is worth telling anybody about:
                // there, what is missing from the run is the finding.
                if setting.declares || setting.built {
                    continue;
                }
                let reaching: Vec<String> = setting
                    .entries
                    .iter()
                    .flat_map(|entry| patterns(&entry.text))
                    .collect();
                // Where nothing but the file's name says the setting is about
                // the run, the value has to say so too: a settings file for the
                // tests holds keys about other things, and a pattern that
                // reaches no suite at all was never about them.
                if !names_the_tests(setting)
                    && !reaching.iter().any(|pattern| {
                        mine.iter()
                            .any(|suite| hides(pattern, directory, &suite.path))
                    })
                {
                    continue;
                }
                narrow(file, setting, &reaching, directory, &mine, into);
            }
            // A switch that stops the runner finding suites for itself leaves
            // the targets the file declares, and nothing else, in the run. A
            // switch on one target says nothing about the others, so it is
            // read as being about that target rather than about the run.
            Sense::Switch(false) if setting.key.to_ascii_lowercase().contains("auto") => {
                narrow(file, setting, &declared(stated), directory, &mine, into);
            }
            Sense::Command if the_default_run(setting) => {
                let mut reaching = Vec::new();
                for entry in &setting.entries {
                    let (excluded, collected) = command(&entry.text);
                    for pattern in excluded {
                        exclude(file, setting, entry.line, &pattern, directory, &mine, into);
                    }
                    // An argument is a path the run collects only where it
                    // reaches a suite. A command line carries all sorts of
                    // words, and one that reaches nothing was never about them.
                    reaching.extend(collected.into_iter().filter(|pattern| {
                        mine.iter()
                            .any(|suite| hides(pattern, directory, &suite.path))
                    }));
                }
                if !reaching.is_empty() {
                    narrow(file, setting, &reaching, directory, &mine, into);
                }
            }
            Sense::Command | Sense::Switch(_) | Sense::Quiet => {}
        }
    }
}

/// Whether the setting's value is built rather than written, which is reported
/// and never guessed at.
fn unreadable(file: &File<'_>, setting: &Stated, into: &mut Collection) -> bool {
    if !setting.built {
        return false;
    }
    into.unreadable.push(Unreadable {
        path: file.path.to_string(),
        line: setting.line,
        setting: setting.key.clone(),
    });
    true
}

/// Every suite one pattern takes out of the run.
fn exclude(
    file: &File<'_>,
    setting: &Stated,
    line: u32,
    pattern: &str,
    directory: &str,
    mine: &[&Suite],
    into: &mut Collection,
) {
    // A pattern that reaches inside a file addresses one case of it, and the
    // file around that case still runs.
    let (pattern, whole) = match pattern.split_once("::") {
        Some((path, _)) => (path, false),
        None => (pattern, true),
    };
    for suite in mine {
        if hides(pattern, directory, &suite.path) {
            into.hidden.push(Hidden {
                test: suite.path.clone(),
                path: file.path.to_string(),
                line,
                setting: setting.key.clone(),
                named: whole,
            });
        }
    }
}

/// Every suite a narrowed pattern leaves outside the run. The line names none
/// of them: what it hides is worked out from everything else the tree holds,
/// which is why it is reported as something to read.
fn narrow(
    file: &File<'_>,
    setting: &Stated,
    reaching: &[String],
    directory: &str,
    mine: &[&Suite],
    into: &mut Collection,
) {
    for suite in mine {
        if reaching
            .iter()
            .any(|pattern| hides(pattern, directory, &suite.path))
        {
            continue;
        }
        into.hidden.push(Hidden {
            test: suite.path.clone(),
            path: file.path.to_string(),
            line: setting.line,
            setting: setting.key.clone(),
            named: false,
        });
    }
}

/// The targets a manifest declares one at a time, which are what is left in the
/// run once it stops finding them itself.
fn declared(stated: &[Stated]) -> Vec<String> {
    stated
        .iter()
        .filter(|setting| setting.declares && setting.scope.iter().any(|held| mentions_tests(held)))
        .flat_map(|setting| {
            let key = setting.key.to_ascii_lowercase();
            setting
                .entries
                .iter()
                .filter_map(move |entry| match key.as_str() {
                    "path" => Some(entry.text.clone()),
                    "name" => Some(format!("tests/{}.rs", entry.text)),
                    _ => None,
                })
        })
        .collect()
}

/// One value as the patterns it holds. A value written as a list is one pattern
/// already; one written as a line of them is as many as it names.
fn patterns(text: &str) -> Vec<String> {
    text.split([' ', '\t', ','])
        .map(|pattern| pattern.trim())
        .filter(|pattern| !pattern.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// What a command line says about the tests it runs: the paths it keeps out,
/// and the paths it collects. An option is read by the words its name is
/// spelled with, the same way a setting's key is.
fn command(text: &str) -> (Vec<String>, Vec<String>) {
    let mut excluded = Vec::new();
    let mut collected = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut at = 0;
    while at < words.len() {
        let word = words[at];
        at += 1;
        let Some(option) = word.strip_prefix("--") else {
            // A path handed to a command is what the command runs; a short
            // option is one weeder cannot tell from its value.
            if !word.starts_with('-') && (word.contains('/') || word.contains('*')) {
                collected.push(word.to_string());
            }
            continue;
        };
        let (name, value) = match option.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => {
                let following = words
                    .get(at)
                    .filter(|following| !following.starts_with('-'))
                    .map(|following| (*following).to_string());
                at += usize::from(following.is_some());
                (option, following)
            }
        };
        let Some(value) = value else {
            continue;
        };
        let name = name.to_ascii_lowercase();
        if EXCLUDING.iter().any(|word| name.contains(word)) {
            excluded.push(value);
        } else if COLLECTING.iter().any(|word| name.contains(word)) {
            collected.push(value);
        }
    }
    (excluded, collected)
}

/// Whether a pattern covers a suite. A pattern is written where the file that
/// holds it sits, so it is read from there as well as from the root.
fn hides(pattern: &str, directory: &str, suite: &str) -> bool {
    reaches(pattern, suite)
        || (!directory.is_empty() && reaches(&format!("{directory}/{pattern}"), suite))
}

fn reaches(pattern: &str, path: &str) -> bool {
    let pattern = pattern
        .trim_start_matches('^')
        .trim_end_matches('$')
        .trim_start_matches("./")
        .trim_matches('/');
    // A pattern may open with a placeholder for wherever the run was started.
    let pattern = match pattern.split_once('>') {
        Some((held, rest)) if held.starts_with('<') => rest.trim_matches('/'),
        _ => pattern,
    };
    if pattern.is_empty() {
        return false;
    }
    if glob::matches(pattern, path) || glob::matches(&format!("{pattern}/**"), path) {
        return true;
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    if !pattern.contains('/') {
        return glob::matches(pattern, name);
    }
    // What is left is a run of whole directories somewhere inside the path,
    // which is how a pattern written for one part of a tree names a file in it.
    inside_path(path, pattern)
}

fn inside_path(path: &str, pattern: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    let wanted: Vec<&str> = pattern.split('/').collect();
    if wanted.len() > segments.len() {
        return false;
    }
    (0..=segments.len() - wanted.len()).any(|from| {
        segments[from..from + wanted.len()]
            .iter()
            .zip(&wanted)
            .all(|(segment, want)| glob::matches(want, segment))
    })
}

/// What one change wrote: the suites its own lines take out of the run, and the
/// settings it wrote that weeder cannot read. A line somebody left in the tree
/// long ago is `scan`'s business; `check` judges what arrived with the change.
#[must_use]
pub fn written_by(collection: &Collection, changes: &[Change]) -> Collection {
    let written: BTreeSet<(&str, u32)> = changes
        .iter()
        .filter_map(|change| Some((change.diff.new_path.as_deref()?, change)))
        .flat_map(|(path, change)| change.added().map(move |(line, _)| (path, line)))
        .collect();
    let wrote = |path: &str, line: u32| written.contains(&(path, line));
    Collection {
        hidden: collection
            .hidden
            .iter()
            .filter(|hidden| wrote(&hidden.path, hidden.line))
            .cloned()
            .collect(),
        unreadable: collection
            .unreadable
            .iter()
            .filter(|unreadable| wrote(&unreadable.path, unreadable.line))
            .cloned()
            .collect(),
    }
}
