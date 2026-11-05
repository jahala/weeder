//! X1, a secret-looking string was added.
//!
//! Two paths to the same finding, and neither of them ever repeats the value it
//! found: a credential printed into a report is a credential published twice.
//!
//! The first path is shape. Issuers stamp their credentials with a prefix and
//! then a long opaque tail, so a token that opens with one of those and runs on
//! is a credential whatever file it landed in. A key block and a signed token
//! carry their own unmistakable frame.
//!
//! The second path is name and disorder together. A quoted literal assigned to
//! something called a key, a secret, a token, a password or a credential, long
//! enough to be one and disordered enough that no person typed it as prose, is a
//! secret. Both halves are needed: a long random string under a plain name is a
//! fixture, a short readable string under a key name is a setting, a sentence
//! under a key name is the message a form shows about one (#93), and the
//! digests filling a lockfile are named for what they are and never for a
//! credential. The literal is needed too, because `token = process.env.TOKEN`
//! reads the credential at run time, which is the safe form, and holds no value.
//!
//! That second path only runs where weeder knows the grammar. Telling a name from
//! a value takes a language that has assignments in it; a page, a stylesheet and
//! a paragraph are full of `name = value` that assigns nothing, a class called
//! `detail-meta__key`, an attribute, a sentence with a colon in the middle of
//! it. So the prefix path reads every file weeder is handed and the name path
//! reads source, where a name means what it says, and the settings files whose
//! every line is a name and a value: env, YAML, TOML and INI (#90). Those are
//! where a secret is committed most often, and they write a value the way their
//! format does, unquoted in env and YAML, so the value is read as written there.
//! A value that names the environment instead, `${VAR}` or `$VAR`, holds none.
//!
//! One list of exact strings stands beside the two shapes: the credentials a
//! vendor prints in its own documentation so a reader can follow the page.
//! Those travel into samples and tests and READMEs, no issuer honours one, and
//! a commit carrying one has nothing to rotate. weeder names it a published
//! example and stands aside, because a gate that stops a manual is a gate an
//! agent learns to walk past. The allowance is the string and not a vendor, so
//! one character away from it is a credential like any other, and the note is
//! what is left when nothing else on the line is one: weeder reports a finding a
//! line, and a note reached first would make the line that quotes a manual the
//! place to hide a key.

use crate::core::change::Change;
use crate::core::classify::Lang;
use crate::core::finding::{Detection, Level, Message, Region, Stamp};
use crate::core::rules::check::Judgement;

/// The prefixes an issuer stamps on a credential, and how much opaque tail one
/// carries before weeder will call it a credential rather than a coincidence.
const PREFIXES: &[&str] = &[
    "AKIA",
    "ASIA",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "github_pat_",
    "sk-",
    "xoxb-",
    "xoxp-",
    "xoxo-",
    "xoxa-",
    "xoxs-",
    "AIza",
    "glpat-",
    "npm_",
];

/// How many characters a prefixed token carries in total before it is long
/// enough to be a credential.
const PREFIXED_LENGTH: usize = 20;

/// The frame a key block opens with, whichever algorithm wrote it.
const KEY_BLOCK_OPEN: &str = "-----BEGIN";
const KEY_BLOCK_KIND: &str = "PRIVATE KEY";

/// What a signed token opens with: the base64 of `{"` , which is how every one
/// of them starts, and the two dots that separate its three parts.
const SIGNED_TOKEN_OPEN: &str = "eyJ";
const SIGNED_TOKEN_PARTS: usize = 3;

/// The words that make a name a name for a credential.
const KEY_WORDS: &[&str] = &[
    "key",
    "keys",
    "secret",
    "secrets",
    "token",
    "tokens",
    "password",
    "passwd",
    "credential",
    "credentials",
];

/// The credentials vendors publish in their own documentation, each written as
/// the stamp an issuer puts on the front and the tail behind it. Joined, they
/// are the strings weeder refuses to carry, so this file carries neither whole and
/// compares against the halves.
///
/// An entry earns its place by being printed in a vendor's own manual, where
/// every scanner already has it and every issuer already refuses it. Nothing is
/// inferred from it: a token that merely looks like an example is a credential.
const PUBLISHED: &[(&str, &str)] = &[
    ("AKIA", "IOSFODNN7EXAMPLE"),
    ("wJalrXUtnFEMI", "/K7MDENG/bPxRfiCYEXAMPLEKEY"),
    ("ghp_", "16C7e42F292c6912E7710c838347Ae178B4a"),
    ("sk_test_", "4eC39HqLyjWDarjtT1zdp7dc"),
];

/// The published examples, joined, for a writer that must not carry one whole:
/// a packet of real hunks quotes whatever the corpus quoted, and a scanner that
/// does not know a textbook from a key reads the packet too. The join happens
/// here at the call and nowhere in a file.
#[must_use]
pub fn published_examples() -> Vec<(String, &'static str)> {
    PUBLISHED
        .iter()
        .map(|(stamp, tail)| (format!("{stamp}{tail}"), *stamp))
        .collect()
}

/// How long a value has to be before disorder means anything, and how disordered
/// it has to be. Four bits a character is above English prose and below the
/// base64 of random bytes.
const ENTROPY_LENGTH: usize = 20;
const ENTROPY_BITS: f64 = 4.0;

/// The characters that make a value a slot to fill rather than a value: nobody
/// issues a credential with a bracket in it.
const SLOT_CHARACTERS: &[char] = &['<', '>', '{', '}'];

/// How many words separated by spaces make a value read as prose. A credential
/// is one run of characters, and a scheme in front of one, `Bearer` and its
/// token, makes two; a sentence long enough to pass for disorder has more.
const PROSE_WORDS: usize = 3;

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let changes = judged.changes;
    let mut findings = Vec::new();
    for change in changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        let grammar = grammar(change, path);
        for (line, text) in change.added() {
            if let Some(shape) = issued(text) {
                findings.push(finding(path, line, &shape));
                continue;
            }
            let named = match grammar {
                Grammar::Source => named_and_disordered(text),
                Grammar::Settings(format) => set_and_disordered(text, format),
                Grammar::None => None,
            };
            if let Some(name) = named {
                findings.push(finding(path, line, &Shape::named(&name)));
                continue;
            }
            // Last, and only where the two paths above found nothing. weeder
            // reports one finding a line, so a note reached before them would
            // be a place to hide a credential: on the line that quotes a manual.
            if quotes_an_example(text) {
                findings.push(finding(path, line, &Shape::published()));
            }
        }
    }
    findings
}

/// What weeder knows of how a file writes a name and its value.
#[derive(Clone, Copy)]
enum Grammar {
    /// A language weeder reads, where a literal is written inside quotes.
    Source,
    /// A settings file, where every line is a name and a value.
    Settings(Format),
    /// Neither: a page, a stylesheet, a paragraph, where `name = value` is a
    /// class name or a colon in a sentence and assigns nothing.
    None,
}

/// The settings formats whose lines X1 reads as a name and a value.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    /// `NAME=value`, after `export` or not, the value quoted or not.
    Env,
    /// `name: value`, the value quoted or not.
    Yaml,
    /// `name = "value"`.
    Toml,
    /// `name = value` or `name: value` under a `[section]`. Python's
    /// `setup.cfg` is written in it too.
    Ini,
}

/// How a changed file writes its names and values, by its language and, for
/// a file in none, by its name.
fn grammar(change: &Change, path: &str) -> Grammar {
    let source = change
        .after
        .classification
        .as_ref()
        .is_some_and(|classification| classification.lang != Lang::Other);
    if source {
        return Grammar::Source;
    }
    settings_format(path).map_or(Grammar::None, Grammar::Settings)
}

/// The settings format a path is written in, by its name.
fn settings_format(path: &str) -> Option<Format> {
    let name = path.rsplit('/').next().unwrap_or(path);
    if name == ".env" || name.starts_with(".env.") || name.ends_with(".env") {
        return Some(Format::Env);
    }
    match name.rsplit_once('.').map(|(_, extension)| extension) {
        Some("yaml" | "yml") => Some(Format::Yaml),
        Some("toml") => Some(Format::Toml),
        Some("ini" | "cfg") => Some(Format::Ini),
        _ => None,
    }
}

/// The credential shape a line carries, named by its frame rather than by its
/// value.
fn issued(line: &str) -> Option<Shape> {
    if line.contains(KEY_BLOCK_OPEN) && line.contains(KEY_BLOCK_KIND) {
        return Some(Shape::key_block());
    }
    for token in tokens(line) {
        // Before the stamp, because a published example wears one, and the
        // line may carry something else that is not an example at all.
        if is_published(token) {
            continue;
        }
        if let Some(prefix) = PREFIXES.iter().find(|prefix| is_issued(token, prefix)) {
            return Some(Shape::prefixed(prefix));
        }
        if is_signed_token(token) {
            return Some(Shape::signed_token());
        }
    }
    None
}

/// Whether a token is one of the examples a vendor published, exactly. The
/// comparison joins nothing: the token is measured against the stamp and then
/// against the tail, so a value one character from an example fails it and is
/// judged as the credential it looks like.
fn is_published(token: &str) -> bool {
    PUBLISHED
        .iter()
        .any(|(stamp, tail)| token.strip_prefix(stamp) == Some(*tail))
}

/// Whether a line quotes one of those examples anywhere in it.
fn quotes_an_example(line: &str) -> bool {
    tokens(line).into_iter().any(is_published)
}

/// The name a line of source assigns a secret-looking value to, where it does.
fn named_and_disordered(line: &str) -> Option<String> {
    assignments(line).into_iter().find_map(|(head, value)| {
        let name = credential_name(head)?;
        is_disordered(value).then_some(name)
    })
}

/// The name a line of a settings file gives a secret-looking value, where it
/// does.
fn set_and_disordered(line: &str, format: Format) -> Option<String> {
    let (head, value) = setting(line, format)?;
    let name = credential_name(head)?;
    is_disordered(value).then_some(name)
}

/// Whether a value assigned to a key-like name reads as a credential: long
/// enough, disordered enough, and none of the things that look like one and
/// are not.
fn is_disordered(value: &str) -> bool {
    value.chars().count() >= ENTROPY_LENGTH
        && !value.contains(SLOT_CHARACTERS)
        // A sentence carries as many kinds of character as a key does, and its
        // spaces are what no issued credential has.
        && value.split_whitespace().count() < PROSE_WORDS
        // An example under a name that says `secret` is still an example; the
        // line is left to the note the caller falls through to.
        && !is_published(value)
        && entropy(value) > ENTROPY_BITS
}

/// The name a settings line sets and the value it sets it to, as the format
/// writes them. A comment, a section header, and a line that opens a list, a
/// table or a block written below set nothing this line can be judged on.
fn setting(line: &str, format: Format) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') || (format == Format::Ini && trimmed.starts_with(';')) {
        return None;
    }
    let trimmed = match format {
        Format::Env => trimmed.strip_prefix("export ").unwrap_or(trimmed),
        Format::Yaml => trimmed.strip_prefix("- ").unwrap_or(trimmed),
        Format::Toml | Format::Ini => trimmed,
    };
    let (at, separator) = match format {
        Format::Env | Format::Toml => (trimmed.find('=')?, 1),
        // A YAML key ends at a colon with a space behind it, so a colon inside
        // a URL or a time is part of a value.
        Format::Yaml => (trimmed.find(": ")?, 2),
        Format::Ini => (trimmed.find(['=', ':'])?, 1),
    };
    let value = written_value(&trimmed[at + separator..], format)?;
    Some((&trimmed[..at], value))
}

/// A settings value as its file writes it: what a quote encloses, or the bare
/// value up to a comment. A value that reads the environment when the file is
/// loaded, `${VAR}`, `$VAR`, or INI's `%(name)s`, holds no credential (#62), and
/// one that opens a list, a table, a tag or a block written below holds none on
/// this line.
fn written_value(tail: &str, format: Format) -> Option<&str> {
    let trimmed = tail.trim();
    let first = trimmed.chars().next()?;
    let value = if matches!(first, '"' | '\'') {
        let opened = first.len_utf8();
        let end = trimmed[opened..].find(first)?;
        &trimmed[opened..opened + end]
    } else {
        if matches!(first, '[' | '{' | '|' | '>' | '!' | '&' | '*') {
            return None;
        }
        let comment = match format {
            Format::Ini => trimmed.find(" ;").or_else(|| trimmed.find(" #")),
            Format::Env | Format::Yaml | Format::Toml => trimmed.find(" #"),
        };
        comment.map_or(trimmed, |at| &trimmed[..at]).trim_end()
    };
    let reference = value.starts_with('$') || value.contains("${") || value.contains("%(");
    (!reference).then_some(value)
}
/// The name a credential was assigned to, where what an assignment was written
/// in front of names one.
///
/// A language may write a keyword and a type around the name, `pub const NAME:
/// &str`, `const name: string`, `"name"`, so every word of the declaration is a
/// candidate, and any one of them reading as a credential is enough. The search
/// stops at the punctuation that ends a statement or opens a list, so the name
/// of one entry never speaks for the next.
fn credential_name(head: &str) -> Option<String> {
    let statement = head
        .rsplit([',', ';', '{', '}', '(', ')', '['])
        .next()
        .unwrap_or(head);
    statement
        .split(|character: char| {
            !(character.is_alphanumeric() || character == '_' || character == '-')
        })
        .find(|name| {
            split_words(name)
                .iter()
                .any(|word| KEY_WORDS.contains(&word.as_str()))
        })
        .map(ToString::to_string)
}

/// A name split into the words it was written from, whatever separated them.
fn split_words(name: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut previous: Option<char> = None;
    for character in name.chars() {
        if !character.is_alphanumeric() {
            words.push(std::mem::take(&mut current));
            previous = None;
            continue;
        }
        if character.is_uppercase()
            && previous.is_some_and(|earlier| earlier.is_lowercase() || earlier.is_numeric())
        {
            words.push(std::mem::take(&mut current));
        } else if character.is_lowercase()
            && previous.is_some_and(char::is_uppercase)
            && current.chars().count() > 1
        {
            // An acronym ends one character before the next word begins, so the
            // capital that started that word goes with it: `APIKey` is `api`
            // and then `key`.
            let carried = current.pop().unwrap_or_default();
            words.push(std::mem::take(&mut current));
            current.push(carried);
        }
        previous = Some(character);
        current.push(character.to_ascii_lowercase());
    }
    words.push(current);
    words.retain(|word| !word.is_empty());
    words
}

/// Every assignment of a literal a line writes: what stands in front of the
/// punctuation that joins them, and the quoted value behind it.
fn assignments(line: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let bytes = line.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        let operator = match byte {
            b':' if bytes.get(index + 1) == Some(&b'=') => 2,
            b':' => 1,
            // A comparison joins nothing, and neither does an operator that
            // updates in place.
            b'=' if bytes.get(index + 1) == Some(&b'=')
                || matches!(
                    bytes.get(index.wrapping_sub(1)),
                    Some(b'=' | b'!' | b'<' | b'>' | b'+' | b'-' | b'*' | b'/' | b'%')
                ) =>
            {
                continue
            }
            b'=' => 1,
            _ => continue,
        };
        let Some(value) = value_after(&line[index + operator..]) else {
            continue;
        };
        found.push((&line[..index], value));
    }
    found
}

/// The value an assignment gives, where it gives a literal: what a quote
/// encloses.
///
/// Anything else on the right is an expression, and an expression holds no
/// credential. `process.env.ANTHROPIC_AUTH_TOKEN` is long and mixed enough to
/// read as one, and it is the read at run time this rule's own advice asks
/// for. Source writes every literal it carries inside quotes, so a value with
/// none around it is a name, a call or a path to a field, and never the secret.
fn value_after(tail: &str) -> Option<&str> {
    let trimmed = tail.trim_start();
    let first = trimmed.chars().next()?;
    if !matches!(first, '"' | '\'' | '`') {
        return None;
    }
    let opened = first.len_utf8();
    let end = trimmed[opened..].find(first)?;
    Some(&trimmed[opened..opened + end])
}

/// The opaque runs a line is made of: the alphabets every credential is written
/// in, and nothing that could end one.
fn tokens(line: &str) -> Vec<&str> {
    line.split(|character: char| !is_token(character))
        .filter(|token| !token.is_empty())
        .collect()
}

/// Whether a token is an issued credential: the issuer's stamp, then a tail
/// long enough and mixed enough to be the opaque part. A name that happens to
/// start the same way, a setting, a path, spells its tail in words alone.
fn is_issued(token: &str, prefix: &str) -> bool {
    let Some(tail) = token.strip_prefix(prefix) else {
        return false;
    };
    token.len() >= PREFIXED_LENGTH
        && tail.chars().any(|character| character.is_ascii_digit())
        && tail
            .chars()
            .any(|character| character.is_ascii_alphabetic())
}

fn is_token(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(character, '_' | '-' | '.' | '+' | '/' | '=' | '~')
}

/// Whether a token is a signed token: three base64 parts, the first of them the
/// encoding of an object, and enough of it to carry a signature.
fn is_signed_token(token: &str) -> bool {
    if !token.starts_with(SIGNED_TOKEN_OPEN) {
        return false;
    }
    let parts: Vec<&str> = token.split('.').collect();
    parts.len() == SIGNED_TOKEN_PARTS
        && parts.iter().all(|part| {
            part.len() >= 8
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        })
}

/// How many bits of surprise each character of a value carries. Prose settles
/// near three; the base64 of random bytes runs above five.
fn entropy(value: &str) -> f64 {
    let characters: Vec<char> = value.chars().collect();
    let total = characters.len() as f64;
    if total == 0.0 {
        return 0.0;
    }
    let mut counts: Vec<(char, usize)> = Vec::new();
    for character in characters {
        match counts.iter_mut().find(|(seen, _)| *seen == character) {
            Some((_, count)) => *count += 1,
            None => counts.push((character, 1)),
        }
    }
    -counts
        .iter()
        .map(|(_, count)| {
            let share = *count as f64 / total;
            share * share.log2()
        })
        .sum::<f64>()
}

/// A credential shape, named without its value ever being repeated, and the
/// level weeder reports that shape at.
struct Shape {
    stamp: Stamp,
    message: Message,
}

impl Shape {
    fn prefixed(prefix: &str) -> Shape {
        Shape::credential(format!(
            "a credential was added: a token stamped `{prefix}` and a long opaque tail."
        ))
    }

    fn key_block() -> Shape {
        Shape::credential("a private key block was added.".to_string())
    }

    fn signed_token() -> Shape {
        Shape::credential(
            "a signed token was added: three base64 parts with a signature on the end.".to_string(),
        )
    }

    fn named(name: &str) -> Shape {
        Shape::credential(format!(
            "a secret-looking value was added, assigned to `{name}`."
        ))
    }

    /// A credential, whatever gave it away: it blocks, and the reader is told
    /// to take it out and rotate it.
    fn credential(what: String) -> Shape {
        Shape {
            stamp: Stamp::Rule,
            message: Message {
                what,
                why: "a credential in a commit is a credential published, and deleting the line later leaves it in the history.".to_string(),
                next: "take the value out, rotate it, and read the credential from the environment at run time; a value that is no credential takes a `Weeder-allow: X1` trailer saying what it is.".to_string(),
            },
        }
    }

    /// An example a vendor printed in its own documentation. It is said out
    /// loud and it stops nothing.
    fn published() -> Shape {
        Shape {
            stamp: Stamp::Override(Level::Note),
            message: Message {
                what: "a published example credential was added: the string a vendor prints in its own documentation.".to_string(),
                why: "an example is a quotation rather than a key, so there is nothing to rotate and nothing to block, and weeder names it here so a reader is not left wondering whether it was read.".to_string(),
                next: "leave it where the documentation is being quoted; anywhere else, take it out, since a value no issuer honours will not work either.".to_string(),
            },
        }
    }
}

fn finding(path: &str, line: u32, shape: &Shape) -> Detection {
    Detection {
        stamp: shape.stamp,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: shape.message.clone(),
        fix: None,
    }
}
