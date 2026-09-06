//! R1, the docs cite something that no longer exists.
//!
//! Documentation rots by staying still while the tree moves. What a document
//! cites is one of four things, and each has an authority: a path resolves
//! against the tree, a command and its flags resolve against the command's own
//! help, and a symbol resolves against everything the repository carries outside
//! its own prose. weed reports a citation only where it can name the authority
//! that refused it.
//!
//! Precision is the whole game here. Prose is full of words that would look like
//! a symbol and slashes that would look like a path, so a citation has to be
//! written in a shape that only code is written in before it is resolved at all:
//! a path ending in an extension the repository writes files in, a first word
//! the config named as a command, an identifier carrying an underscore, a case
//! change or a call's parentheses. Everything else is prose, and prose is not
//! this rule's business.
//!
//! One thing weed cannot see: which repository a sentence is about. A document
//! describing another tool cites that tool's files and names, and they resolve
//! against this tree and fail. The shape tests above cut most of it, a
//! placeholder, an elision, a directory this tree has never had, and what is
//! left is why `[rules] R1 = "off"` exists.

use std::collections::BTreeSet;

use crate::core::config::Config;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::glob;
use crate::core::syntax;
use crate::core::tree::{CommandListing, Tree};

use super::{code_words, declared_names};

/// The documents a repository states itself in. A path is repository-relative,
/// and `docs/**/*.md` reaches every depth under the directory.
const DOCUMENTS: &[&str] = &["CLAUDE.md", "AGENTS.md", "README.md", "docs/**/*.md"];

/// Whether a path is one of the documents a repository states itself in.
fn is_document(path: &str) -> bool {
    DOCUMENTS.iter().any(|pattern| glob::matches(pattern, path))
}

pub fn evaluate(tree: &Tree, _config: &Config) -> Vec<Finding> {
    let extensions = tree.extensions();
    let known = known_identifiers(tree);
    let mut findings = Vec::new();

    for file in &tree.files {
        if !is_document(&file.path) {
            continue;
        }
        for citation in citations(file.text()) {
            let unresolved = match &citation.kind {
                Kind::Path => path_complaint(tree, &citation.text, &extensions),
                Kind::Command => command_complaint(tree, &citation.text),
                Kind::Symbol => symbol_complaint(&known, &citation.text),
            };
            for complaint in unresolved {
                findings.push(finding(&file.path, citation.line, &complaint));
            }
        }
    }
    findings
}

/// Every name the repository still carries, outside the prose being judged.
///
/// A symbol resolves against what the code declares, what the code writes, and
/// what the repository's data holds, a vendored schema, a manifest, the map.
/// The documents R1 reads are the one place it does not look: a citation that
/// could vouch for itself, or for the one in the paragraph above it, would never
/// go stale and would never be worth reading.
fn known_identifiers(tree: &Tree) -> BTreeSet<String> {
    let mut known = declared_names(tree);
    known.extend(code_words(tree).into_keys());
    for file in &tree.files {
        if file.is_code() || is_document(&file.path) {
            continue;
        }
        known.extend(
            syntax::words(file.text())
                .into_iter()
                .map(|word| word.text.to_string()),
        );
    }
    known
}

/// What a document cited, and what kind of thing it is.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Citation {
    kind: Kind,
    text: String,
    /// 1-based, the line of the document the citation is written on.
    line: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Path,
    Command,
    Symbol,
}

/// What could not be resolved, and against what.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Complaint {
    what: String,
    why: String,
    next: String,
}

fn finding(path: &str, line: u32, complaint: &Complaint) -> Finding {
    Finding {
        rule: "R1".to_string(),
        level: Level::Warn,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: complaint.what.clone(),
            why: complaint.why.clone(),
            next: complaint.next.clone(),
        },
        fix: None,
        suppressed: None,
    }
}

/// Whether a cited path is still there.
///
/// A citation that ends in a file extension names a file, and it is resolved
/// wherever it points: a file that moved is the thing this rule exists to
/// report. A citation that ends in a directory name is far weaker, prose is
/// full of `and/or`, of branch names and of another tool's directories, so one
/// of those is resolved only when its first segment is something this tree has.
/// A path with a glob in it resolves on the directory it starts from, because
/// weed matches globs against paths and a pattern that matches nothing today may
/// be the point of the sentence.
fn path_complaint(tree: &Tree, cited: &str, extensions: &BTreeSet<String>) -> Vec<Complaint> {
    let cited = &normalize(cited);
    let trimmed = cited.trim_end_matches('/');
    if let Some(prefix) = literal_prefix(trimmed) {
        if prefix.is_empty() || !about_this_tree(tree, &prefix) || tree.holds_under(&prefix) {
            return Vec::new();
        }
        return vec![Complaint {
            what: format!("the docs cite `{cited}`, and nothing in the tree is under `{prefix}`."),
            why: "a pattern rooted at a directory that is gone matches nothing, so whoever follows the doc finds an empty answer rather than an error.".to_string(),
            next: format!("point it at the directory `{prefix}` became, or drop the sentence."),
        }];
    }
    if trimmed.contains('/') {
        let named_file = extension_of(trimmed).is_some();
        if (!named_file && !about_this_tree(tree, trimmed))
            || tree.holds(trimmed)
            || tree.holds_under(trimmed)
        {
            return Vec::new();
        }
        return vec![Complaint {
            what: format!("the docs cite the path `{cited}`, and the tree holds nothing there."),
            why: "a reader following the path finds nothing, and a tool given it fails on a file that has moved or gone.".to_string(),
            next: "cite where the file is now, or delete the reference.".to_string(),
        }];
    }
    // A bare name with an extension the repository writes files in is a file,
    // wherever it sits.
    let extension = trimmed
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());
    if !extension.is_some_and(|extension| extensions.contains(&extension))
        || tree.holds_name(trimmed)
    {
        return Vec::new();
    }
    vec![Complaint {
        what: format!("the docs cite the file `{cited}`, and no file in the tree carries that name."),
        why: "a reader looking for the file finds none, and the doc describes a repository that no longer exists.".to_string(),
        next: "cite the file the repository has, or delete the reference.".to_string(),
    }]
}

/// The extension a path's last segment ends in, lowercased. `None` where the
/// segment carries none, and where the only dot opens it, a dotfile and a
/// directory both name a place rather than a file.
fn extension_of(path: &str) -> Option<String> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let (stem, extension) = name.rsplit_once('.')?;
    if stem.is_empty()
        || extension.is_empty()
        || !extension
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return None;
    }
    Some(extension.to_ascii_lowercase())
}

/// Whether a path's first segment is something this tree carries, which is what
/// makes it a path into this repository rather than into one the document is
/// describing.
fn about_this_tree(tree: &Tree, path: &str) -> bool {
    let Some(first) = path.split('/').next() else {
        return false;
    };
    !first.is_empty() && (tree.holds(first) || tree.holds_under(first))
}

/// A citation with the parts a document wraps around a path taken back off: the
/// anchor a markdown link ends with, and the line or line range a reference to a
/// place inside a file carries.
fn normalize(cited: &str) -> String {
    let without_anchor = cited.split('#').next().unwrap_or(cited);
    match without_anchor.rsplit_once(':') {
        Some((path, tail))
            if !tail.is_empty()
                && tail
                    .chars()
                    .all(|character| character.is_ascii_digit() || character == '-') =>
        {
            path.to_string()
        }
        _ => without_anchor.to_string(),
    }
}

/// The part of a pattern before its first glob character, cut back to a
/// directory. `None` where the text carries no glob at all.
fn literal_prefix(pattern: &str) -> Option<String> {
    let first = pattern.find(['*', '?', '['])?;
    let literal = &pattern[..first];
    Some(match literal.rsplit_once('/') {
        Some((directory, _)) => directory.to_string(),
        None => String::new(),
    })
}

/// Whether a cited command still spells itself that way: every subcommand it
/// walks through, and every flag it passes, against the command's own help.
fn command_complaint(tree: &Tree, cited: &str) -> Vec<Complaint> {
    let words: Vec<&str> = cited.split_whitespace().collect();
    let Some(first) = words.first() else {
        return Vec::new();
    };
    let mut path = vec![first.to_string()];
    let Some(mut listing) = tree.listing(&path) else {
        return Vec::new();
    };
    let mut reachable: Vec<&CommandListing> = vec![listing];
    let mut complaints = Vec::new();

    let mut index = 1;
    while let Some(word) = words.get(index) {
        if word.starts_with('-') {
            break;
        }
        let mut deeper = path.clone();
        deeper.push((*word).to_string());
        match tree.listing(&deeper) {
            Some(found) => {
                path = deeper;
                listing = found;
                reachable.push(found);
                index += 1;
            }
            None => {
                // A command that offers subcommands and does not offer this one
                // is being cited as something it is not. A command that offers
                // none at all is being given an argument, which is not weed's
                // to judge.
                if !listing.subcommands.is_empty() {
                    complaints.push(Complaint {
                        what: format!("the docs cite `{cited}`, and `{}` has no subcommand `{word}`.", path.join(" ")),
                        why: "the command as written fails the moment anyone runs it, and the doc is the only place it still exists.".to_string(),
                        next: format!("run `{} --help` and cite a subcommand it prints.", path.join(" ")),
                    });
                }
                break;
            }
        }
    }

    let known: BTreeSet<&str> = reachable
        .iter()
        .flat_map(|listing| listing.flags.iter().map(String::as_str))
        .collect();
    for word in &words[index.min(words.len())..] {
        let Some(flag) = spelled_flag(word) else {
            continue;
        };
        if known.contains(flag) {
            continue;
        }
        complaints.push(Complaint {
            what: format!("the docs cite `{cited}`, and `{}` has no flag `{flag}`.", path.join(" ")),
            why: "the command as written is refused the moment anyone runs it, and a reader has no way to tell that from the doc.".to_string(),
            next: format!("run `{} --help` and cite a flag it prints.", path.join(" ")),
        });
    }
    complaints
}

/// The flag a word spells, or `None` where the word is not one: a value that
/// happens to open with a dash, a bare `--`, a negative number.
fn spelled_flag(word: &str) -> Option<&str> {
    let flag = word.split_once('=').map_or(word, |(flag, _)| flag);
    let rest = flag.strip_prefix("--").or_else(|| flag.strip_prefix('-'))?;
    rest.chars()
        .next()
        .is_some_and(char::is_alphabetic)
        .then_some(flag)
}

/// Whether a cited symbol is still anywhere in the code.
fn symbol_complaint(known: &BTreeSet<String>, cited: &str) -> Vec<Complaint> {
    let name = cited
        .trim_end_matches("()")
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(cited);
    if name.is_empty() || known.contains(name) {
        return Vec::new();
    }
    vec![Complaint {
        what: format!("the docs cite the symbol `{cited}`, and the code declares and uses no `{name}`."),
        why: "a name that only the documentation still knows sends a reader looking for code that was renamed or deleted.".to_string(),
        next: "cite the name the code carries now, or delete the reference.".to_string(),
    }]
}

/// Every citation a document makes, in the order it makes them.
///
/// Fenced blocks are samples rather than sentences, they hold shell sessions,
/// output and code from other repositories, so what is read here is the inline
/// code spans and the paths written in the prose around them.
fn citations(document: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    let mut fence: Option<String> = None;
    for (index, line) in document.lines().enumerate() {
        let number = index as u32 + 1;
        let trimmed = line.trim_start();
        if let Some(open) = &fence {
            if trimmed.starts_with(open.as_str()) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_opener(trimmed) {
            fence = Some(open);
            continue;
        }
        let (spans, prose) = split_spans(line);
        for span in spans {
            for (kind, text) in span_citations(&span) {
                found.push(Citation {
                    kind,
                    text,
                    line: number,
                });
            }
        }
        for token in prose_paths(&prose) {
            found.push(Citation {
                kind: Kind::Path,
                text: token,
                line: number,
            });
        }
    }
    found
}

/// The run of backticks or tildes a fenced block opens with, or `None` where
/// the line opens none.
fn fence_opener(line: &str) -> Option<String> {
    for character in ['`', '~'] {
        let run: String = line
            .chars()
            .take_while(|found| *found == character)
            .collect();
        if run.len() >= 3 {
            return Some(run);
        }
    }
    None
}

/// A line split into what its code spans hold and what is left of the prose. A
/// span opens on a run of backticks and closes on a run of the same length; an
/// unclosed run is not a span, and what follows it is still prose.
fn split_spans(line: &str) -> (Vec<String>, String) {
    let characters: Vec<char> = line.chars().collect();
    let mut spans = Vec::new();
    let mut prose = String::new();
    let mut index = 0;
    while index < characters.len() {
        if characters[index] != '`' {
            prose.push(characters[index]);
            index += 1;
            continue;
        }
        let opener = run_length(&characters, index);
        match closing(&characters, index + opener, opener) {
            Some(close) => {
                spans.push(characters[index + opener..close].iter().collect::<String>());
                index = close + opener;
            }
            None => {
                prose.extend(&characters[index..index + opener]);
                index += opener;
            }
        }
    }
    (spans, prose)
}

fn run_length(characters: &[char], from: usize) -> usize {
    characters[from..]
        .iter()
        .take_while(|character| **character == '`')
        .count()
}

/// Where the run of exactly `width` backticks that closes a span begins.
fn closing(characters: &[char], from: usize, width: usize) -> Option<usize> {
    let mut index = from;
    while index < characters.len() {
        if characters[index] == '`' {
            let run = run_length(characters, index);
            if run == width {
                return Some(index);
            }
            index += run;
        } else {
            index += 1;
        }
    }
    None
}

/// What a code span cites, and under which authority. A span cites nothing weed
/// can resolve more often than it cites something, and an empty answer is the
/// common one.
fn span_citations(span: &str) -> Vec<(Kind, String)> {
    let span = span.trim();
    if span.is_empty() {
        return Vec::new();
    }
    if span.split_whitespace().count() > 1 {
        return vec![(Kind::Command, span.to_string())];
    }
    if span.starts_with('-') {
        return Vec::new();
    }
    // A place inside a file: the path, then the name of the thing in it. Both
    // halves have an authority, and both are worth asking.
    if let Some((path, symbol)) = span.split_once("::") {
        if path.contains('/') && looks_like_path(path) {
            let mut found = vec![(Kind::Path, path.to_string())];
            if looks_like_symbol(symbol) {
                found.push((Kind::Symbol, symbol.to_string()));
            }
            return found;
        }
    }
    if looks_like_path(span) {
        return vec![(Kind::Path, span.to_string())];
    }
    if looks_like_symbol(span) {
        return vec![(Kind::Symbol, span.to_string())];
    }
    Vec::new()
}

/// Whether a single-word span names a place: it carries a separator, or it
/// carries a dot with something after it that could be an extension. Whether
/// that extension is one this repository uses is decided when it is resolved.
fn looks_like_path(span: &str) -> bool {
    // A placeholder is a shape rather than a place, an elision is a path
    // somebody chose not to write out, and a URL, a home directory and an
    // absolute path all name something outside this repository.
    if span.contains('<')
        || span.contains('>')
        || span.starts_with("...")
        || span.contains("://")
        || span.contains('@')
        || span.starts_with('/')
        || span.starts_with('~')
    {
        return false;
    }
    if span.contains('/') {
        return true;
    }
    span.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty()
            && !extension.is_empty()
            && extension
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
    })
}

/// Whether a single-word span names a symbol.
///
/// Every language weed reads spells an identifier the same way, and prose does
/// too: `check`, `warn` and `off` are words in a sentence as often as they are
/// names in a program. What prose does not write is an underscore inside a
/// word, a capital in the middle of one, or a pair of parentheses on the end,
/// so a span carrying one of those is a symbol and a span carrying none of them
/// is left alone.
fn looks_like_symbol(span: &str) -> bool {
    let called = span.ends_with("()");
    let body = span.trim_end_matches("()");
    let segments: Vec<&str> = body.split("::").flat_map(|part| part.split('.')).collect();
    if segments.is_empty() || segments.iter().any(|segment| !is_identifier(segment)) {
        return false;
    }
    let Some(last) = segments.last() else {
        return false;
    };
    called
        || last.contains('_')
        || (last.chars().any(char::is_uppercase) && last.chars().any(char::is_lowercase))
}

fn is_identifier(segment: &str) -> bool {
    let mut characters = segment.chars();
    characters
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && characters.all(|character| character.is_alphanumeric() || character == '_')
}

/// The paths written in prose rather than in a span. A token has to carry a
/// separator and end in an extension to be one: `and/or` and `fire/silent` are
/// English, and `docs/sarif.md` is a place.
fn prose_paths(prose: &str) -> Vec<String> {
    prose
        .split_whitespace()
        .map(trim_punctuation)
        .filter(|token| {
            token.contains('/')
                && looks_like_path(token)
                && token.rsplit_once('.').is_some_and(|(_, extension)| {
                    !extension.is_empty()
                        && extension.len() <= 8
                        && extension
                            .chars()
                            .all(|character| character.is_ascii_alphanumeric())
                })
        })
        .map(ToString::to_string)
        .collect()
}

/// A token with the punctuation a sentence wrapped it in taken back off.
fn trim_punctuation(token: &str) -> &str {
    token
        .trim_start_matches(['(', '[', '{', '"', '\'', '<'])
        .trim_end_matches([')', ']', '}', '"', '\'', '>', ',', ';', ':', '.', '!', '?'])
}
