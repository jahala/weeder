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
//! The second path is name and disorder together. A value assigned to something
//! called a key, a secret, a token, a password or a credential, long enough to be
//! one and disordered enough that no person typed it as prose, is a secret. Both
//! halves are needed: a long random string under a plain name is a fixture, a
//! short readable string under a key name is a setting, and the digests filling
//! a lockfile are named for what they are and never for a credential.
//!
//! That second path only runs where weed knows the grammar. Telling a name from
//! a value takes a language that has assignments in it; a page, a stylesheet and
//! a paragraph are full of `name = value` that assigns nothing, a class called
//! `detail-meta__key`, an attribute, a sentence with a colon in the middle of
//! it. So the prefix path reads every file weed is handed and the name path
//! reads source alone, which is the only place a name means what it says.
//!
//! One list of exact strings stands beside the two shapes: the credentials a
//! vendor prints in its own documentation so a reader can follow the page.
//! Those travel into samples and tests and READMEs, no issuer honours one, and
//! a commit carrying one has nothing to rotate. weed names it a published
//! example and stands aside, because a gate that stops a manual is a gate an
//! agent learns to walk past. The allowance is the string and not a vendor, so
//! one character away from it is a credential like any other, and the note is
//! what is left when nothing else on the line is one: weed reports a finding a
//! line, and a note reached first would make the line that quotes a manual the
//! place to hide a key.

use crate::core::change::Change;
use crate::core::classify::Lang;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::rules::check::Judgement;

/// The prefixes an issuer stamps on a credential, and how much opaque tail one
/// carries before weed will call it a credential rather than a coincidence.
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
/// are the strings weed refuses to carry, so this file carries neither whole and
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

pub fn evaluate(judged: &Judgement) -> Vec<Finding> {
    let changes = judged.changes;
    let mut findings = Vec::new();
    for change in changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        let source = is_source(change);
        for (line, text) in change.added() {
            if let Some(shape) = issued(text) {
                findings.push(finding(path, line, &shape));
                continue;
            }
            if source {
                if let Some(name) = named_and_disordered(text) {
                    findings.push(finding(path, line, &Shape::named(&name)));
                    continue;
                }
            }
            // Last, and only where the two paths above found nothing. weed
            // reports one finding a line, so a note reached before them would
            // be a place to hide a credential: on the line that quotes a manual.
            if quotes_an_example(text) {
                findings.push(finding(path, line, &Shape::published()));
            }
        }
    }
    findings
}

/// Whether the file is written in a language weed reads. Outside one, weed has
/// no grammar to tell an assignment from a class name or a colon in a sentence.
fn is_source(change: &Change) -> bool {
    change
        .after
        .classification
        .as_ref()
        .is_some_and(|classification| classification.lang != Lang::Other)
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

/// The name a line assigns a secret-looking value to, where it does.
fn named_and_disordered(line: &str) -> Option<String> {
    for (head, value) in assignments(line) {
        let Some(name) = credential_name(head) else {
            continue;
        };
        if value.chars().count() < ENTROPY_LENGTH {
            continue;
        }
        if value.contains(SLOT_CHARACTERS) {
            continue;
        }
        // An example under a name that says `secret` is still an example; the
        // line is left to the note the caller falls through to.
        if is_published(value) {
            continue;
        }
        if entropy(value) <= ENTROPY_BITS {
            continue;
        }
        return Some(name);
    }
    None
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

/// Every assignment a line writes: what stands in front of the punctuation that
/// joins them, and the value behind it. The value is a quoted literal where the
/// format quotes, and the bare word where it does not.
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

/// The value an assignment gives: what a quote encloses, or the bare word.
fn value_after(tail: &str) -> Option<&str> {
    let trimmed = tail.trim_start();
    let mut characters = trimmed.char_indices();
    let (_, first) = characters.next()?;
    if matches!(first, '"' | '\'' | '`') {
        let opened = first.len_utf8();
        let end = trimmed[opened..].find(first)?;
        return Some(&trimmed[opened..opened + end]);
    }
    let end = trimmed
        .find(|character: char| !is_token(character))
        .unwrap_or(trimmed.len());
    (end > 0).then(|| &trimmed[..end])
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
/// level weed reports that shape at.
struct Shape {
    level: Level,
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
            level: Level::Block,
            message: Message {
                what,
                why: "a credential in a commit is a credential published, and deleting the line later leaves it in the history.".to_string(),
                next: "take the value out, rotate it, and read the credential from the environment at run time.".to_string(),
            },
        }
    }

    /// An example a vendor printed in its own documentation. It is said out
    /// loud and it stops nothing.
    fn published() -> Shape {
        Shape {
            level: Level::Note,
            message: Message {
                what: "a published example credential was added: the string a vendor prints in its own documentation.".to_string(),
                why: "an example is a quotation rather than a key, so there is nothing to rotate and nothing to block, and weed names it here so a reader is not left wondering whether it was read.".to_string(),
                next: "leave it where the documentation is being quoted; anywhere else, take it out, since a value no issuer honours will not work either.".to_string(),
            },
        }
    }
}

fn finding(path: &str, line: u32, shape: &Shape) -> Finding {
    Finding {
        rule: "X1".to_string(),
        level: shape.level,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: shape.message.clone(),
        fix: None,
        suppressed: None,
    }
}
