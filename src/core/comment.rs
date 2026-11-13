//! weeder's verdict as the markdown of one pull-request comment.
//!
//! The body is rendered from the SARIF log itself, so the comment and the log a
//! machine reads can never disagree about what was found. It opens with a
//! hidden marker, which is how the action finds the comment it wrote last time
//! and edits it rather than writing a second one. Then one sentence when there
//! is nothing to report, or a count and a list: blocks first, then warnings,
//! then notes, each with its level, its rule, the place it names linked to that
//! line, and its message.
//!
//! A link needs the server, the repository and the commit, and core reads none
//! of them from anywhere: the caller hands in the address the files of that
//! commit sit under. Without one, a place is written as code and links nowhere.

use crate::core::sarif::{counted, rank, Log, SarifLevel, SarifResult};

/// The first line of every body: html a reader never sees, and the line the
/// action looks for at the top of a comment to know it is the one to edit.
pub const MARKER: &str = "<!-- weeder: one comment, edited in place on every push -->";

/// The log as a comment. `links` is the address a path is appended to, such as
/// `https://github.com/owner/repo/blob/<sha>`.
pub fn render(log: &Log, links: Option<&str>) -> String {
    let links = links.map(|base| base.trim_end_matches('/'));
    let mut body = format!("{MARKER}\n\n");

    let failure = log
        .runs
        .iter()
        .flat_map(|run| &run.invocations)
        .find(|invocation| !invocation.execution_successful);
    if let Some(invocation) = failure {
        let reasons: Vec<String> = invocation
            .tool_execution_notifications
            .iter()
            .flatten()
            .map(|notification| paragraph(&notification.message.text))
            .collect();
        body.push_str("weeder could not judge this pull request, so nothing in it was cleared.");
        for reason in reasons {
            body.push(' ');
            body.push_str(&reason);
        }
        body.push('\n');
        return body;
    }

    let mut results: Vec<&SarifResult> = log.runs.iter().flat_map(|run| &run.results).collect();
    if results.is_empty() {
        body.push_str("weeder found nothing to report in this pull request.\n");
        return body;
    }
    // The log is already in its order law, file, line, rule and message; a
    // stable sort by level keeps that order inside each level.
    results.sort_by_key(|result| rank(result.level));

    let count = |level: SarifLevel| results.iter().filter(|r| r.level == level).count();
    let errors = count(SarifLevel::Error);
    let counts: Vec<String> = [
        (errors, "error"),
        (count(SarifLevel::Warning), "warning"),
        (count(SarifLevel::Note), "note"),
    ]
    .iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, word)| counted(*count, word))
    .collect();
    body.push_str(&format!("weeder found {}. ", listed(&counts)));
    body.push_str(if errors > 0 {
        "An error blocks this pull request, and each finding below says what to do next.\n\n"
    } else {
        "Nothing here blocks this pull request; each finding below is for whoever reviews it.\n\n"
    });

    for result in results {
        body.push_str(&format!(
            "- **{}** {} {}\\\n  {}\n",
            result.level.word(),
            code(&result.rule_id),
            place(result, links),
            paragraph(&result.message.text),
        ));
    }
    body
}

/// `a`, `a and b`, `a, b and c`.
fn listed(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The place a result names, shown as written and linked to its line where
/// the caller said where the files are.
fn place(result: &SarifResult, links: Option<&str>) -> String {
    let Some(location) = result.locations.first() else {
        return String::new();
    };
    let path = &location.physical_location.artifact_location.uri;
    let region = location.physical_location.region.as_ref();
    let shown = match region {
        Some(region) => format!("{path}:{}", region.start_line),
        None => path.clone(),
    };
    // A note about the change as a whole names the repository as `.`, and a
    // blob address has no file there to show, so it links nowhere.
    let Some(base) = links.filter(|_| path != ".") else {
        return code(&shown);
    };
    let fragment = match region {
        Some(region) => match region.end_line {
            Some(end) => format!("#L{}-L{end}", region.start_line),
            None => format!("#L{}", region.start_line),
        },
        None => String::new(),
    };
    format!("[{}]({base}/{}{fragment})", code(&shown), encoded(path))
}

/// A path as a URL writes it: every byte but the unreserved ones and `/`
/// percent-encoded, so a space, a bracket or a parenthesis cannot end the link.
fn encoded(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Text as a code span, fenced with one backtick more than the longest run it
/// holds, so no backtick inside it can close it early.
fn code(text: &str) -> String {
    let mut longest = 0;
    let mut run = 0;
    for character in text.chars() {
        if character == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    let fence = "`".repeat(longest + 1);
    if text.starts_with('`') || text.ends_with('`') {
        format!("{fence} {text} {fence}")
    } else {
        format!("{fence}{text}{fence}")
    }
}

/// A message as one paragraph that reads as it was written. Its code spans,
/// the backtick runs a reader would see closed, stay code; everything between
/// them is text, escaped where markdown would read it as something else.
fn paragraph(message: &str) -> String {
    let flat = message.split_whitespace().collect::<Vec<&str>>().join(" ");
    let mut out = String::new();
    let characters: Vec<char> = flat.chars().collect();
    let mut index = 0;
    let mut plain = String::new();
    while index < characters.len() {
        if characters[index] != '`' {
            plain.push(characters[index]);
            index += 1;
            continue;
        }
        let width = characters[index..]
            .iter()
            .take_while(|character| **character == '`')
            .count();
        let after = index + width;
        match closing(&characters, after, width) {
            Some(close) => {
                out.push_str(&text(&plain));
                plain.clear();
                out.extend(&characters[index..close + width]);
                index = close + width;
            }
            None => {
                plain.extend(&characters[index..after]);
                index = after;
            }
        }
    }
    out.push_str(&text(&plain));
    lead(&out)
}

/// Where a run of exactly `width` backticks starts at or after `from`.
fn closing(characters: &[char], from: usize, width: usize) -> Option<usize> {
    let mut index = from;
    while index < characters.len() {
        if characters[index] != '`' {
            index += 1;
            continue;
        }
        let run = characters[index..]
            .iter()
            .take_while(|character| **character == '`')
            .count();
        if run == width {
            return Some(index);
        }
        index += run;
    }
    None
}

/// Text markdown would otherwise read: emphasis, links, html, entities, a
/// backtick nobody closed. A name after an `@` goes into code, where GitHub
/// notifies nobody of it.
fn text(plain: &str) -> String {
    let characters: Vec<char> = plain.chars().collect();
    let mut out = String::with_capacity(plain.len());
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character == '@' && characters.get(index + 1).is_some_and(|next| handle(*next)) {
            let mut end = characters[index + 1..]
                .iter()
                .position(|next| !handle(*next) && !matches!(next, '.' | '/'))
                .map_or(characters.len(), |at| index + 1 + at);
            // A name ends before the full stop that ends its sentence. The
            // character after the `@` is part of the name, so this stops there.
            while matches!(characters[end - 1], '.' | '/') {
                end -= 1;
            }
            let name: String = characters[index..end].iter().collect();
            out.push_str(&code(&name));
            index = end;
            continue;
        }
        if matches!(
            character,
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '&' | '~' | '|' | '#'
        ) {
            out.push('\\');
        }
        out.push(character);
        index += 1;
    }
    out
}

fn handle(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
}

/// A paragraph that would open a list, a heading or a rule where it starts a
/// line has that first character escaped.
fn lead(paragraph: &str) -> String {
    let digits = paragraph.chars().take_while(char::is_ascii_digit).count();
    let after = paragraph[digits..].chars().next();
    if digits > 0 && matches!(after, Some('.' | ')')) {
        return format!("{}\\{}", &paragraph[..digits], &paragraph[digits..]);
    }
    match paragraph.chars().next() {
        Some('-' | '+' | '=') => format!("\\{paragraph}"),
        _ => paragraph.to_string(),
    }
}
