//! What the repository is built out of: which types are built on which, and
//! which of them an entry file states to the world.
//!
//! A method that says it is not implemented reads two ways, and which one it is
//! depends on code that is nowhere near it. Written in a leaf it is a stub: a
//! caller reaches it and the program fails. Written in a base it is a contract,
//! and the types built on it are what fill it in. Only the tree can tell the two
//! apart, so a rule judging such a method asks here which types are built on the
//! one it is written in, and what each of those writes.
//!
//! Nothing here parses a language. The reader hands over an outline, and what a
//! declaration is built on is read off the one line that declares it, through
//! the syntax mask, so a base named in a comment or spelled inside a string says
//! nothing. Each language states the relation its own way, `class Fast(Strategy)`,
//! `class Fast extends Strategy`, `impl Sink for Buffer`, and each name is read
//! down to its last segment, so a base reached through its module answers for
//! itself. Go states it nowhere: a Go type carries the methods and the compiler
//! draws the conclusion, so a Go file names no bases here and a rule asking
//! about one gets no answer rather than a guess.

use std::collections::{BTreeMap, BTreeSet};

use crate::core::classify::Lang;
use crate::core::read::{Definition, DefinitionKind, Outline};
use crate::core::syntax::{words, Mask};

/// One type in the tree built on a type a rule asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    /// The type it is built on, by the last segment of the name its declaration
    /// gave.
    pub base: String,
    /// The methods it writes itself.
    pub methods: Vec<String>,
    /// Whether the file that declares it carries tests. A suite's own subclass
    /// stands in for nothing a caller reaches, so it fills no contract in.
    pub test: bool,
}

/// What the tree answered about the types a rule asked about. Empty where
/// nothing asked: reading a repository happens only where a rule has a question
/// for it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Hierarchy {
    /// Every type in the tree built on one of the types asked about, in the
    /// order the tree's paths were read, so two runs answer the same.
    pub derived: Vec<Derived>,
    /// The types an entry file states, each with the entry file that states it.
    pub exported: BTreeMap<String, String>,
}

impl Hierarchy {
    /// Whether production code somewhere in the tree is built on `base` and
    /// writes `method` itself.
    #[must_use]
    pub fn overrides(&self, base: &str, method: &str) -> bool {
        self.derived.iter().any(|derived| {
            !derived.test
                && derived.base == base
                && derived.methods.iter().any(|written| written == method)
        })
    }

    /// The entry file that states `base`, where one does.
    #[must_use]
    pub fn exported_through(&self, base: &str) -> Option<&str> {
        self.exported.get(base).map(String::as_str)
    }
}

/// Whether a language states in a declaration what a type is built on. A file
/// written in a language that does not is one a caller need never read.
#[must_use]
pub fn states_its_bases(lang: Lang) -> bool {
    matches!(
        lang,
        Lang::TypeScript | Lang::JavaScript | Lang::Python | Lang::Rust
    )
}

/// Whether a path is the file a package states itself through: a crate's
/// `lib.rs`, a directory's `index`, a Python package's `__init__.py`. What such
/// a file names is what a consumer outside this repository can reach.
#[must_use]
pub fn is_entry(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name == "lib.rs"
        || name == "__init__.py"
        || matches!(name.split_once('.'), Some(("index", extension)) if !extension.is_empty())
}

/// Every type one file declares that is built on one of `wanted`, with the
/// methods it writes.
#[must_use]
pub fn derived_in(
    wanted: &BTreeSet<String>,
    lang: Lang,
    test: bool,
    outline: &Outline,
    mask: &Mask,
) -> Vec<Derived> {
    let mut found = Vec::new();
    for definition in outline.flatten() {
        if !can_be_built_on_something(lang, definition, mask) {
            continue;
        }
        let methods: Vec<String> = definition
            .children
            .iter()
            .filter(|child| child.kind == DefinitionKind::Function)
            .map(|child| child.name.clone())
            .collect();
        for base in bases(lang, &mask.code(definition.start_line)) {
            if wanted.contains(&base) {
                found.push(Derived {
                    base,
                    methods: methods.clone(),
                    test,
                });
            }
        }
    }
    found
}

/// Which of `wanted` a file names in its code. This is what makes an entry file
/// an export: the name is written where the language reads it, rather than in a
/// comment about it or in a string that happens to spell it.
#[must_use]
pub fn stated_in(wanted: &BTreeSet<String>, mask: &Mask) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for line in 1..=mask.line_count() {
        let code = mask.code(line);
        for word in words(&code) {
            if wanted.contains(word.text) && !found.iter().any(|seen| seen == word.text) {
                found.push(word.text.to_string());
            }
        }
    }
    found
}

/// The type a definition is written inside, the innermost one where they nest,
/// or `None` where it sits in no type at all. This is the type whose contract
/// the definition states, and the one another type can be built on.
#[must_use]
pub fn enclosing_type<'a>(outline: &'a Outline, definition: &Definition) -> Option<&'a Definition> {
    outline
        .flatten()
        .into_iter()
        .filter(|candidate| {
            states_a_contract(candidate)
                && candidate.start_line <= definition.start_line
                && definition.end_line <= candidate.end_line
        })
        .max_by_key(|candidate| candidate.start_line)
}

/// The innermost function a line is written inside.
#[must_use]
pub fn enclosing_function(outline: &Outline, line: u32) -> Option<&Definition> {
    outline
        .flatten()
        .into_iter()
        .filter(|candidate| candidate.kind == DefinitionKind::Function && candidate.spans(line))
        .max_by_key(|candidate| candidate.start_line)
}

/// Whether a definition is a type another type can be built on.
///
/// A Rust `impl` block is not one. It is where a type's own methods are written,
/// nothing is built on it, and a body left unwritten there is left unwritten for
/// the one type it names.
fn states_a_contract(definition: &Definition) -> bool {
    matches!(
        definition.kind,
        DefinitionKind::Class | DefinitionKind::Interface | DefinitionKind::Struct
    )
}

/// Whether a definition is one whose declaration can name what it is built on:
/// a class, an interface, a struct, or the block Rust writes an implementation
/// in, which is the only place Rust states the relation at all.
fn can_be_built_on_something(lang: Lang, definition: &Definition, mask: &Mask) -> bool {
    states_a_contract(definition)
        || (lang == Lang::Rust
            && definition.kind == DefinitionKind::Module
            && opens_with(&mask.code(definition.start_line), "impl"))
}

/// What a declaration says it is built on, each base by the last segment of its
/// name, so a base reached through its module answers for itself.
#[must_use]
pub fn bases(lang: Lang, declaration: &str) -> Vec<String> {
    let named = match lang {
        Lang::Python => python_bases(declaration),
        Lang::TypeScript | Lang::JavaScript => script_bases(declaration),
        Lang::Rust => rust_bases(declaration),
        Lang::Go | Lang::Other => Vec::new(),
    };
    named
        .iter()
        .map(|base| last_segment(base).to_string())
        .filter(|base| !base.is_empty())
        .collect()
}

/// `class Fast(Strategy, metaclass=Meta)`: whatever the parentheses hold. A base
/// given as a keyword argument is read from the value it names.
fn python_bases(declaration: &str) -> Vec<String> {
    let Some(open) = declaration.find('(') else {
        return Vec::new();
    };
    let close = declaration[open..]
        .rfind(')')
        .map_or(declaration.len(), |end| open + end);
    declaration[open + 1..close]
        .split(',')
        .map(|base| base.rsplit('=').next().unwrap_or(base).trim().to_string())
        .collect()
}

/// `class Fast extends Strategy implements Sink, Store`: everything named after
/// either keyword, up to the brace that opens the body.
fn script_bases(declaration: &str) -> Vec<String> {
    const CLAUSES: [&str; 2] = ["extends", "implements"];
    let head = cut_at(declaration, '{');
    let mut found = Vec::new();
    for clause in CLAUSES {
        let Some(at) = word_start(head, clause) else {
            continue;
        };
        let listed = &head[at + clause.len()..];
        let listed = CLAUSES
            .iter()
            .filter_map(|other| word_start(listed, other))
            .min()
            .map_or(listed, |end| &listed[..end]);
        found.extend(listed.split(',').map(|name| name.trim().to_string()));
    }
    found
}

/// `impl Sink for Buffer`, the trait an implementation is written for, and
/// `trait Faster: Sink`, the traits a trait is written on top of. An inherent
/// `impl Buffer` is written on nothing.
fn rust_bases(declaration: &str) -> Vec<String> {
    let declaration = strip_visibility(declaration.trim());
    if let Some(rest) = after_keyword(declaration, "impl") {
        let rest = past_generics(rest);
        let Some(at) = outside_generics(rest, "for") else {
            return Vec::new();
        };
        return vec![rest[..at].trim().to_string()];
    }
    let Some(rest) = after_keyword(declaration, "trait") else {
        return Vec::new();
    };
    // The trait's own name, then whatever generics it takes, and only then the
    // colon that opens the list: a bound written inside the generics carries a
    // colon of its own.
    let rest = past_generics(rest.trim_start().trim_start_matches(is_name));
    let Some(list) = rest.trim_start().strip_prefix(':') else {
        return Vec::new();
    };
    let list = cut_at(list, '{');
    let list = list.split(" where ").next().unwrap_or(list);
    list.split('+')
        .map(|base| base.trim().to_string())
        .collect()
}

/// What follows a keyword written as the first word of a declaration, or
/// `None` where the declaration opens with something else.
fn after_keyword<'a>(declaration: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = declaration.trim_start().strip_prefix(keyword)?;
    (!rest.starts_with(is_name)).then_some(rest)
}

/// A Rust declaration with the visibility in front of it taken off.
fn strip_visibility(declaration: &str) -> &str {
    let rest = declaration.trim_start();
    let Some(rest) = rest.strip_prefix("pub") else {
        return rest;
    };
    if rest.starts_with(is_name) {
        return declaration.trim_start();
    }
    let rest = rest.trim_start();
    match rest.strip_prefix('(') {
        Some(inside) => inside
            .split_once(')')
            .map_or(rest, |(_, tail)| tail)
            .trim_start(),
        None => rest,
    }
}

/// The text after the generic parameters a declaration opens with, where it
/// opens with any.
fn past_generics(declaration: &str) -> &str {
    let rest = declaration.trim_start();
    if !rest.starts_with('<') {
        return rest;
    }
    let mut depth = 0usize;
    for (at, character) in rest.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return &rest[at + character.len_utf8()..];
                }
            }
            _ => {}
        }
    }
    ""
}

/// Where a word is written, outside every generic argument list, so the `for`
/// of a higher-ranked bound is not read as the `for` of an implementation.
fn outside_generics(text: &str, wanted: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut read = 0usize;
    for word in words(text) {
        depth += depth_of(&text[read..word.start]);
        read = word.end;
        if word.text == wanted && depth == 0 {
            return Some(word.start);
        }
    }
    None
}

/// Where a word is written in a line, as a word rather than as letters inside
/// a longer one.
fn word_start(text: &str, wanted: &str) -> Option<usize> {
    words(text)
        .into_iter()
        .find(|word| word.text == wanted)
        .map(|word| word.start)
}

/// How far into a generic argument list a stretch of punctuation goes.
fn depth_of(punctuation: &str) -> i32 {
    punctuation
        .chars()
        .map(|character| match character {
            '<' => 1,
            '>' => -1,
            _ => 0,
        })
        .sum()
}

/// Whether a line opens with this word.
fn opens_with(line: &str, keyword: &str) -> bool {
    after_keyword(line, keyword).is_some()
}

/// The text before the first of a character, or all of it where it holds none.
fn cut_at(text: &str, character: char) -> &str {
    text.split_once(character).map_or(text, |(head, _)| head)
}

/// A name with its generic arguments taken off.
fn without_generics(name: &str) -> &str {
    cut_at(name, '<')
}

/// The last segment of a dotted or a pathed name.
fn last_segment(name: &str) -> &str {
    let name = without_generics(name).trim();
    let name = name.rsplit("::").next().unwrap_or(name);
    name.rsplit('.').next().unwrap_or(name).trim()
}

fn is_name(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}
