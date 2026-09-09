//! R1, the docs cite something that no longer exists.
//!
//! Documentation rots by staying still while the tree moves. What a document
//! cites is one of four things, and each has an authority: a path resolves
//! against the tree, a command and its flags resolve against the command's own
//! help, and a symbol resolves against everything the repository carries outside
//! its own prose. weeder reports a citation only where it can name the authority
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
//! A citation is read whole. The `:line` or `:start-end` written on the end of
//! a path is part of what the document claimed, so a path that resolves is
//! still answered: a line past the end of the file is reported with the length
//! the file actually has. And a name is read with the paragraph around it. A
//! sentence that names a file or a directory and then a symbol has pinned that
//! name to that place, so the place answers first and the whole tree second,
//! and the finding names the place a reader should go and look at. A name with
//! nothing beside it is answered by the tree alone and reported as a note,
//! which is how the fifteen findings on the document somebody is reading stay
//! visible above the two hundred on words that were never symbols.
//!
//! One thing weeder cannot see: which repository a sentence is about. A document
//! describing another tool cites that tool's files and names, and they resolve
//! against this tree and fail. The shape tests above cut most of it, a
//! placeholder, an elision, a directory this tree has never had, and what is
//! left is why `[rules] R1 = "off"` exists.

use std::collections::BTreeSet;

use crate::core::config::Config;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::glob;
use crate::core::syntax;
use crate::core::tree::{CommandListing, Tree, TreeFile};

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
        let citations = citations(file.text());
        // The paths first, and every one of them, because what a path resolved
        // to is the authority for the names written beside it. A name read
        // before the paragraph's path had been resolved would be answered by
        // the whole repository when the paragraph had named a smaller place.
        let mut anchors: Vec<Option<Anchor>> = vec![None; citations.len()];
        for (index, citation) in citations.iter().enumerate() {
            if citation.kind != Kind::Path {
                continue;
            }
            match place(tree, &citation.text, &extensions) {
                Place::Found(anchor) => {
                    if let Some(complaint) = line_complaint(&anchor, &citation.text) {
                        findings.push(finding(&file.path, citation.line, &complaint));
                    }
                    anchors[index] = Some(anchor);
                }
                Place::Gone(complaint) => {
                    findings.push(finding(&file.path, citation.line, &complaint));
                }
                Place::Prose => {}
            }
        }
        for (index, citation) in citations.iter().enumerate() {
            let unresolved = match &citation.kind {
                Kind::Path => continue,
                Kind::Command => command_complaint(tree, &citation.text),
                Kind::Symbol => symbol_complaint(
                    &known,
                    anchoring(&citations, &anchors, index),
                    &citation.text,
                ),
            };
            for complaint in unresolved {
                findings.push(finding(&file.path, citation.line, &complaint));
            }
        }
    }
    findings
}

/// The place a name is answerable to: the path citation standing nearest to it.
///
/// A sentence names a file and then says what is in it, or says what is in a
/// file and then names it, so either side of the same line will do. Failing
/// that, the last path the section resolved is what the paragraph is still
/// about. A heading ends that, and a path that resolves to nothing anchors
/// nothing, because a place that is not there cannot answer for a name.
fn anchoring<'a>(
    citations: &[Citation],
    anchors: &'a [Option<Anchor<'a>>],
    index: usize,
) -> Option<&'a Anchor<'a>> {
    let here = &citations[index];
    let anchored = |at: &usize| anchors[*at].is_some();
    let before = || (0..index).rev();
    let after = || index + 1..citations.len();
    before()
        .find(|at| citations[*at].line == here.line && anchored(at))
        .or_else(|| after().find(|at| citations[*at].line == here.line && anchored(at)))
        .or_else(|| before().find(|at| citations[*at].section == here.section && anchored(at)))
        .and_then(|at| anchors[at].as_ref())
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
    /// Which run of the document it sits in, counted from the headings above
    /// it. Two citations under the same heading are about the same thing; two
    /// either side of one need not be.
    section: u32,
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
    /// How loudly this one is worth saying. A citation whose paragraph pinned
    /// it to a place is a claim weeder can hold to that place; a name standing
    /// on its own is answered by the whole repository, and prose writes those
    /// by the hundred, so it is reported and left quiet.
    level: Level,
    what: String,
    why: String,
    next: String,
}

fn finding(path: &str, line: u32, complaint: &Complaint) -> Finding {
    Finding {
        rule: "R1".to_string(),
        level: complaint.level,
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

/// What the tree says about a cited path.
enum Place<'a> {
    /// The tree answers for it, and this is what it answered with.
    Found(Anchor<'a>),
    /// The tree holds nothing there, and here is why that matters.
    Gone(Complaint),
    /// Not a citation this rule resolves: a shape prose writes as often as
    /// code does.
    Prose,
}

/// A place a document named, and everything that place holds. This is what a
/// name written beside it is answered by.
#[derive(Debug, Clone)]
struct Anchor<'a> {
    /// The files the citation reaches: the one it names, or every file under
    /// the directory it names.
    files: Vec<&'a TreeFile>,
    /// Whether it named a directory. That is a wider claim than a file, and the
    /// finding says so rather than pretending the document named one file.
    directory: bool,
    /// The place as the finding should name it.
    display: String,
}

/// Whether a cited path is still there, and what it points at when it is.
///
/// A citation that ends in a file extension names a file, and it is resolved
/// wherever it points: a file that moved is the thing this rule exists to
/// report. A citation that ends in a directory name is far weaker, prose is
/// full of `and/or`, of branch names and of another tool's directories, so one
/// of those is resolved only when its first segment is something this tree has.
/// A path with a glob in it resolves on the directory it starts from, because
/// weeder matches globs against paths and a pattern that matches nothing today may
/// be the point of the sentence.
fn place<'a>(tree: &'a Tree, cited: &str, extensions: &BTreeSet<String>) -> Place<'a> {
    let (written, _) = split_place(cited);
    let trimmed = written.trim_end_matches('/');
    if let Some(prefix) = literal_prefix(trimmed) {
        if prefix.is_empty() || !about_this_tree(tree, &prefix) {
            return Place::Prose;
        }
        if tree.holds_under(&prefix) {
            return Place::Found(under(tree, &prefix, &prefix));
        }
        return Place::Gone(Complaint {
            level: Level::Warn,
            what: format!("the docs cite `{cited}`, and nothing in the tree is under `{prefix}`."),
            why: "a pattern rooted at a directory that is gone matches nothing, so whoever follows the doc finds an empty answer rather than an error.".to_string(),
            next: format!("point it at the directory `{prefix}` became, or drop the sentence."),
        });
    }
    // The slash a document ends a directory with is part of how it named the
    // place, so what carries one is read as a place and never as a bare name.
    if written.contains('/') {
        if let Some(file) = tree.files.iter().find(|file| file.path == trimmed) {
            return Place::Found(Anchor {
                files: vec![file],
                directory: false,
                display: file.path.clone(),
            });
        }
        if tree.holds_under(trimmed) {
            return Place::Found(under(tree, trimmed, written));
        }
        let named_file = extension_of(trimmed).is_some();
        if !named_file && !about_this_tree(tree, trimmed) {
            return Place::Prose;
        }
        return Place::Gone(Complaint {
            level: Level::Warn,
            what: format!("the docs cite the path `{cited}`, and the tree holds nothing there."),
            why: "a reader following the path finds nothing, and a tool given it fails on a file that has moved or gone.".to_string(),
            next: "cite where the file is now, or delete the reference.".to_string(),
        });
    }
    // A bare name with an extension the repository writes files in is a file,
    // wherever it sits.
    let extension = trimmed
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());
    if !extension.is_some_and(|extension| extensions.contains(&extension)) {
        return Place::Prose;
    }
    let carried: Vec<&TreeFile> = tree
        .files
        .iter()
        .filter(|file| file.name() == trimmed)
        .collect();
    if !carried.is_empty() {
        return Place::Found(Anchor {
            files: carried,
            directory: false,
            display: trimmed.to_string(),
        });
    }
    Place::Gone(Complaint {
        level: Level::Warn,
        what: format!("the docs cite the file `{cited}`, and no file in the tree carries that name."),
        why: "a reader looking for the file finds none, and the doc describes a repository that no longer exists.".to_string(),
        next: "cite the file the repository has, or delete the reference.".to_string(),
    })
}

/// Everything the tree holds under a directory, as the place a document named.
fn under<'a>(tree: &'a Tree, prefix: &str, display: &str) -> Anchor<'a> {
    let root = format!("{}/", prefix.trim_end_matches('/'));
    Anchor {
        files: tree
            .files
            .iter()
            .filter(|file| file.path.starts_with(&root))
            .collect(),
        directory: true,
        display: display.to_string(),
    }
}

/// The line a citation into a resolved file names, when the file is not that
/// long. A document pointing at line 479 of a file with 163 lines in it is as
/// wrong as one pointing at a file that is gone, and it is wrong in a way a
/// reader cannot tell from the page.
fn line_complaint(anchor: &Anchor, cited: &str) -> Option<Complaint> {
    let (_, lines) = split_place(cited);
    let (_, last) = lines?;
    // A directory has no lines of its own, and a bare name several files answer
    // to gives no one file to count.
    if anchor.directory {
        return None;
    }
    let [file] = anchor.files[..] else {
        return None;
    };
    // A file weeder could read no text out of has no lines to be past the end of.
    let count = file.content.as_ref()?.lines().count() as u32;
    if last <= count {
        return None;
    }
    let path = &file.path;
    Some(Complaint {
        level: Level::Warn,
        what: format!("the docs cite `{cited}`, and `{path}` is {count} lines long."),
        why: "a citation past the end of a file lands nowhere, and the lines it was written against have moved somewhere this one does not say.".to_string(),
        next: format!("read `{path}` and cite the line the text sits on now, or drop the number."),
    })
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

/// A citation taken apart into the place it names and the lines it names inside
/// it: the anchor a markdown link ends with comes off, and the `:line` or
/// `:start-end` is handed back rather than dropped, because where the path
/// resolves the line is the other half of what the document claimed.
fn split_place(cited: &str) -> (&str, Option<(u32, u32)>) {
    let without_anchor = cited.split('#').next().unwrap_or(cited);
    match without_anchor.rsplit_once(':') {
        Some((path, tail))
            if !tail.is_empty()
                && tail
                    .chars()
                    .all(|character| character.is_ascii_digit() || character == '-') =>
        {
            (path, span_of(tail))
        }
        _ => (without_anchor, None),
    }
}

/// The first and last line a citation's tail names, or `None` where the tail is
/// digits and dashes without being a line or a range: `1-2-3` is a name and
/// `-4` is not a place in a file.
fn span_of(tail: &str) -> Option<(u32, u32)> {
    match tail.split_once('-') {
        Some((start, end)) => Some((start.parse().ok()?, end.parse().ok()?)),
        None => {
            let only = tail.parse().ok()?;
            Some((only, only))
        }
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
                // none at all is being given an argument, which is not weeder's
                // to judge.
                if !listing.subcommands.is_empty() {
                    complaints.push(Complaint {
                        level: Level::Warn,
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
            level: Level::Warn,
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

/// Whether a cited symbol is still anywhere in the code, and how loudly to say
/// that it is not.
///
/// A paragraph that names a place and then a symbol has made one claim, not
/// two: that the place declares the name. weeder asks the place first, the whole
/// repository second, and reports what neither answered at warning level naming
/// the place, because a reader can go and look. A name with no place beside it
/// is a word in a sentence as often as it is a symbol, so it is asked of the
/// repository alone and reported as a note: still there for anyone reading the
/// whole log, and never the loudest thing in it.
fn symbol_complaint(
    known: &BTreeSet<String>,
    anchor: Option<&Anchor>,
    cited: &str,
) -> Vec<Complaint> {
    let name = cited
        .trim_end_matches("()")
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(cited);
    if name.is_empty() {
        return Vec::new();
    }
    let Some(anchor) = anchor else {
        if known.contains(name) {
            return Vec::new();
        }
        return vec![Complaint {
            level: Level::Note,
            what: format!("the docs cite the symbol `{cited}`, and the code declares and uses no `{name}`."),
            why: "a name that only the documentation still knows sends a reader looking for code that was renamed or deleted.".to_string(),
            next: "cite the name the code carries now, or delete the reference.".to_string(),
        }];
    };
    let declared = anchor
        .files
        .iter()
        .any(|file| file.outline.find(name).is_some());
    if declared || known.contains(name) {
        return Vec::new();
    }
    let place = if anchor.directory {
        format!("the directory `{}`", anchor.display)
    } else {
        format!("the file `{}`", anchor.display)
    };
    vec![Complaint {
        level: Level::Warn,
        what: format!("the docs cite `{cited}` beside {place}, and neither it nor the rest of the tree declares `{name}`."),
        why: "a name the paragraph pins to a place is a claim about that place, and a reader who goes there finds nothing of the kind.".to_string(),
        next: format!("cite the name {place} carries now, or delete the reference."),
    }]
}

/// Every citation a document makes, in the order it makes them.
///
/// Fenced blocks are samples rather than sentences, they hold shell sessions,
/// output and code from other repositories, so what is read here is the inline
/// code spans and the paths written in the prose around them.
///
/// The order is the document's own, spans and prose interleaved as they are
/// written, because a name is answered by the path standing nearest to it and
/// nearest is a question about where the words sit on the page.
fn citations(document: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    let mut fence: Option<String> = None;
    let mut section = 0;
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
        if is_heading(trimmed) {
            section += 1;
        }
        for piece in split_line(line) {
            let cited = match piece {
                Piece::Span(span) => span_citations(&span),
                Piece::Prose(prose) => prose_paths(&prose)
                    .into_iter()
                    .map(|token| (Kind::Path, token))
                    .collect(),
            };
            for (kind, text) in cited {
                found.push(Citation {
                    kind,
                    text,
                    line: number,
                    section,
                });
            }
        }
    }
    found
}

/// Whether a line opens a section. A heading ends whatever the paragraphs above
/// it were about, so it is where a name stops being answerable to the path the
/// last paragraph named.
fn is_heading(line: &str) -> bool {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    (1..=6).contains(&hashes)
        && line[hashes..]
            .chars()
            .next()
            .is_none_or(char::is_whitespace)
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

/// One run of a line: what a code span holds, or the prose between two of them.
enum Piece {
    Span(String),
    Prose(String),
}

/// A line split into the runs it is written from, in the order it writes them.
/// A span opens on a run of backticks and closes on a run of the same length;
/// an unclosed run is not a span, and what follows it is still prose.
fn split_line(line: &str) -> Vec<Piece> {
    let characters: Vec<char> = line.chars().collect();
    let mut pieces = Vec::new();
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
                if !prose.is_empty() {
                    pieces.push(Piece::Prose(std::mem::take(&mut prose)));
                }
                pieces.push(Piece::Span(
                    characters[index + opener..close].iter().collect::<String>(),
                ));
                index = close + opener;
            }
            None => {
                prose.extend(&characters[index..index + opener]);
                index += opener;
            }
        }
    }
    if !prose.is_empty() {
        pieces.push(Piece::Prose(prose));
    }
    pieces
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

/// What a code span cites, and under which authority. A span cites nothing weeder
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
/// Every language weeder reads spells an identifier the same way, and prose does
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
