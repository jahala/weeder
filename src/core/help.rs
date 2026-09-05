//! What a command accepts, read off its own `--help`.
//!
//! A repository's docs cite the commands the repository is about, and the only
//! authority on whether a subcommand or a flag still exists is the command
//! itself. Help output is not a format anybody standardised, but the shape the
//! generators emit is the same one: a heading that ends in `Commands:` over
//! indented lines whose first word is a subcommand, and a heading of `Options:`
//! or `Flags:` over indented lines that open with the flag's spellings. That
//! shape is all this reads, so a command built with clap and one built with
//! cobra are both understood, and anything else simply lists nothing.

/// What one help listing offers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Listing {
    pub subcommands: Vec<String>,
    pub flags: Vec<String>,
}

/// The word a generator prints for its own help entry. It is furniture rather
/// than a command of the program, and a doc never cites it as one.
const HELP_ENTRY: &str = "help";

/// One command's help, read for the subcommands and flags it names.
#[must_use]
pub fn parse(text: &str) -> Listing {
    let mut listing = Listing::default();
    for (heading, lines) in sections(text) {
        if is_commands_heading(&heading) {
            for line in &lines {
                if let Some(name) = subcommand(line) {
                    push(&mut listing.subcommands, name);
                }
            }
        } else if is_options_heading(&heading) {
            for line in &lines {
                for flag in flags(line) {
                    push(&mut listing.flags, flag);
                }
            }
        }
    }
    listing
}

/// Every heading in the help and the indented lines under it. A section ends at
/// the first line that is not indented, which is where the next heading or the
/// end of the output is.
fn sections(text: &str) -> Vec<(String, Vec<String>)> {
    let mut found: Vec<(String, Vec<String>)> = Vec::new();
    for line in text.lines() {
        let indented = line.starts_with(' ') || line.starts_with('\t');
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !indented && trimmed.ends_with(':') {
            found.push((trimmed.to_string(), Vec::new()));
        } else if indented {
            if let Some((_, lines)) = found.last_mut() {
                lines.push(trimmed.to_string());
            }
        } else {
            // Prose at the left margin — the description a help opens with —
            // closes whatever section was open.
            found.push((String::new(), Vec::new()));
        }
    }
    found
        .into_iter()
        .filter(|(heading, _)| !heading.is_empty())
        .collect()
}

fn is_commands_heading(heading: &str) -> bool {
    let lower = heading.to_ascii_lowercase();
    lower.ends_with("commands:")
}

fn is_options_heading(heading: &str) -> bool {
    let lower = heading.to_ascii_lowercase();
    lower.ends_with("options:") || lower.ends_with("flags:")
}

/// The subcommand an entry names: its first word, where that word is a name
/// rather than a flag or a continuation of the line above.
fn subcommand(line: &str) -> Option<&str> {
    let word = line.split_whitespace().next()?;
    if word.starts_with('-') || word == HELP_ENTRY {
        return None;
    }
    word.chars()
        .next()
        .is_some_and(|character| character.is_alphanumeric())
        .then_some(word)
}

/// The flag spellings an entry opens with: every leading word that starts with
/// a dash, up to the value name or the description. A spelling written with its
/// value attached (`--format=<kind>`) is the flag without the value.
fn flags(line: &str) -> Vec<String> {
    line.split_whitespace()
        .take_while(|word| word.starts_with('-'))
        .map(|word| {
            let word = word.trim_end_matches(',');
            word.split_once('=')
                .map_or(word, |(flag, _)| flag)
                .to_string()
        })
        .filter(|flag| flag.len() > 1)
        .collect()
}

fn push(into: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if !into.contains(&value) {
        into.push(value);
    }
}
