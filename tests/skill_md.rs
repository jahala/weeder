//! The skill an agent reads instead of an instruction block. Two things make it
//! worth installing: a description short enough to sit in a listing without
//! costing a paragraph, and a body that covers the whole binary. The second is
//! the one that rots, so it is checked against the binary rather than against a
//! list written down beside it, every command and every flag weeder prints, all
//! the way down the tree, has to appear in SKILL.md.
//!
//! Coverage is not the whole of it. An allowance is the one thing an agent has
//! to get right by reading rather than by trying, because the wrong half of it,
//! a marker the agent writes on its own line, is honoured by no hook at any
//! stage. So the two pages that teach it, SKILL.md and `docs/rules.md`, are held
//! to naming the stage that reads a trailer and the stage that only names it.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use common::weeder_command_in;
use weeder::core::suppress::TRAILER;

/// The stage that reads an allowance a person wrote, and the stage that runs
/// before one exists. A page that teaches the flow has to name both, or an agent
/// reading it writes the allowance where nothing reads it.
const READS_THE_TRAILER: &str = "guard commit-msg";
const NAMES_THE_TRAILER: &str = "guard pre-commit";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn skill() -> String {
    let path = root().join("SKILL.md");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("SKILL.md should be readable: {error}"))
}

/// The yaml front matter, as key and value, and the body below it. A skill is a
/// markdown file that opens with a fenced block of metadata; anything else is
/// not a skill a harness can install.
fn front_matter(source: &str) -> (Vec<(String, String)>, String) {
    let lines: Vec<&str> = source.lines().collect();
    assert_eq!(
        lines.first(),
        Some(&"---"),
        "SKILL.md should open with yaml front matter"
    );
    let close = lines
        .iter()
        .position(|line| line.trim() == "---")
        .and_then(|_| lines.iter().skip(1).position(|line| line.trim() == "---"))
        .map(|offset| offset + 1)
        .expect("the front matter should close with ---");

    let fields = lines[1..close]
        .iter()
        .map(|line| {
            let (key, value) = line.split_once(':').unwrap_or_else(|| {
                panic!("front matter line `{line}` should be a key and a value")
            });
            (key.trim().to_string(), value.trim().to_string())
        })
        .collect();
    (fields, lines[close + 1..].join("\n"))
}

/// How many sentences a piece of prose carries. A sentence ends at `.`, `?` or
/// `!`, at the end of the text, or before whitespace and the capital that
/// opens the next one.
fn sentences(text: &str) -> usize {
    let characters: Vec<char> = text.trim().chars().collect();
    let mut count = 0;
    for (index, character) in characters.iter().enumerate() {
        if !matches!(character, '.' | '?' | '!') {
            continue;
        }
        let rest = &characters[index + 1..];
        let next = rest.iter().find(|character| !character.is_whitespace());
        match next {
            None => count += 1,
            Some(next) if next.is_uppercase() && rest[0].is_whitespace() => count += 1,
            _ => {}
        }
    }
    count
}

/// Every command and every flag the binary prints, walked from `weeder --help`
/// down through each subcommand's own help. clap's `help` command is its
/// furniture rather than a face of weeder, so the walk steps around it.
fn printed_surface() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    walk(&[], &mut found);
    assert!(
        found.iter().any(|name| !name.starts_with('-')),
        "weeder --help printed no commands"
    );
    found
}

fn walk(path: &[String], found: &mut BTreeSet<String>) {
    let mut arguments: Vec<&str> = path.iter().map(String::as_str).collect();
    arguments.push("--help");
    let printed = weeder_command_in(&root(), &arguments)
        .output()
        .expect("the weeder binary should run");
    let text = String::from_utf8_lossy(&printed.stdout).to_string();

    for flag in section(&text, "Options:")
        .iter()
        .flat_map(|line| flags(line))
    {
        found.insert(flag);
    }
    for command in section(&text, "Commands:")
        .iter()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_string)
        .collect::<Vec<String>>()
    {
        found.insert(command.clone());
        let mut deeper = path.to_vec();
        deeper.push(command);
        walk(&deeper, found);
    }
}

/// The indented lines under a heading in clap's help, up to the blank line that
/// ends the section.
fn section<'a>(text: &'a str, heading: &str) -> Vec<&'a str> {
    text.lines()
        .skip_while(|line| line.trim() != heading)
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim)
        .collect()
}

/// The flag spellings an option line opens with: every leading word that starts
/// with a dash, up to the value name or the description.
fn flags(line: &str) -> Vec<String> {
    line.split_whitespace()
        .take_while(|word| word.starts_with('-'))
        .map(|word| word.trim_end_matches(',').to_string())
        .collect()
}

#[test]
fn the_description_is_one_sentence() {
    let (fields, _) = front_matter(&skill());
    let description = fields
        .iter()
        .find(|(key, _)| key == "description")
        .map(|(_, value)| value.clone())
        .expect("SKILL.md's front matter should carry a description");
    assert_eq!(
        sentences(&description),
        1,
        "the description should be one sentence, and it is: {description}"
    );
}

#[test]
fn the_body_names_every_command_and_flag_the_binary_prints() {
    let source = skill();
    let (_, body) = front_matter(&source);
    let missing: Vec<String> = printed_surface()
        .into_iter()
        .filter(|name| !names(&body, name))
        .collect();
    assert!(
        missing.is_empty(),
        "the binary prints these, and SKILL.md never names them: {}",
        missing.join(" ")
    );
}

/// Whether the body names something the binary prints. A name has to stand as
/// its own word: `--base` inside `--base-ref` is a different flag, and `check`
/// inside `checked` is prose.
fn names(body: &str, name: &str) -> bool {
    let boundary = |character: char| !character.is_alphanumeric() && character != '-';
    body.match_indices(name).any(|(index, _)| {
        let before = body[..index].chars().next_back().is_none_or(boundary);
        let after = body[index + name.len()..]
            .chars()
            .next()
            .is_none_or(boundary);
        before && after
    })
}

/// A markdown section: its heading, and every line under it up to the next
/// heading of the same rank or above. It is how a page is held to teaching
/// something *where* it teaches it, rather than anywhere in the file.
fn section_named(source: &str, heading: &str) -> String {
    let rank = heading
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let mut lines = source.lines().skip_while(|line| line.trim_end() != heading);
    let opening = lines
        .next()
        .unwrap_or_else(|| panic!("the page should carry a section headed `{heading}`"));
    let body = lines.take_while(|line| match rank_of(line) {
        Some(deeper) => deeper > rank,
        None => true,
    });
    std::iter::once(opening)
        .chain(body)
        .collect::<Vec<&str>>()
        .join("\n")
}

/// How deep a markdown heading sits, or `None` for a line that is not one.
fn rank_of(line: &str) -> Option<usize> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    (hashes > 0 && line[hashes..].starts_with(' ')).then_some(hashes)
}

/// Every term a page has to carry where it teaches an allowance through, read
/// off the binary's own token rather than off a copy kept here. The page is read
/// as one run of words, because where a sentence was wrapped is the writer's
/// business and not the reader's.
fn taught(page: &str, where_it_is: &str) {
    let words = page.split_whitespace().collect::<Vec<&str>>().join(" ");
    for term in [
        READS_THE_TRAILER,
        NAMES_THE_TRAILER,
        &format!("{TRAILER} <RULE> <reason>"),
    ] {
        assert!(
            words.contains(term),
            "{where_it_is} does not name `{term}`, and an allowance written where no stage reads \
             it is an agent talking to itself:\n{page}"
        );
    }
}

#[test]
fn the_skill_teaches_where_a_trailer_is_written_and_which_stage_reads_it() {
    let source = skill();
    let (_, body) = front_matter(&source);
    taught(
        &section_named(&body, "### The trailer flow"),
        "SKILL.md's trailer flow",
    );
}

#[test]
fn the_rules_page_says_the_same_where_it_explains_allowances() {
    let path = root().join("docs/rules.md");
    let page = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("docs/rules.md should be readable: {error}"));
    taught(
        &section_named(&page, "## Allowing a finding through"),
        "docs/rules.md's allowance section",
    );
}
