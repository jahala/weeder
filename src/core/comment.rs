//! weeder's verdict as the markdown of one pull-request comment.
//!
//! The body is rendered from the findings, read through the same three parts
//! the table and the log read (`sarif::parts`), so the comment and the log a
//! machine reads can never disagree about what was found. It opens with a
//! hidden marker, which is how the action finds the comment it wrote last time
//! and edits it rather than writing a second one. Then one sentence when there
//! is nothing to report, or a count and a list, blocks first, then warnings,
//! then notes. Each finding opens with a headline, its rule's short
//! description, its level and its rule id linked to the rule's section; under
//! it the place it names, linked to that line, then what was found, why it
//! matters and the next action, a line each; then what was said to allow it,
//! and a link to report it as wrong. The body ends with the command that judges
//! the change the same way on a person's own machine.
//!
//! A link needs the server, the repository and the commit, and core reads none
//! of them from anywhere: the caller hands in the address the files of that
//! commit sit under, and weeder's own home and version. Without the address a
//! place is written as code and links nowhere.

use crate::core::catalogue;
use crate::core::finding::{Finding, Level};
use crate::core::review::Approval;
use crate::core::sarif::{counted, help_uri, parts, rank, sarif_level};
use crate::core::suppress::{Suppression, SuppressionSource, MARKER as LINE_MARKER, TRAILER};

/// The first line of every body: html a reader never sees, and the line the
/// action looks for at the top of a comment to know it is the one to edit.
pub const MARKER: &str = "<!-- weeder: one comment, edited in place on every push -->";

/// The issue form a report of a false positive opens, in weeder's own
/// repository.
pub const REPORT_TEMPLATE: &str = "false-positive.yml";

/// What a comment is written for, beyond the findings: the caller knows all of
/// it and core reads none of it from anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page<'a> {
    /// The version that judged, which the rule links and the report name.
    pub version: &'a str,
    /// weeder's own repository, such as `https://github.com/jahala/weeder`.
    pub home: &'a str,
    /// The address a finding's path is appended to, such as
    /// `https://github.com/owner/repo/blob/<sha>`.
    pub links: Option<&'a str>,
    /// The command that judges the same change the same way on a person's own
    /// machine.
    pub command: &'a str,
    /// The approval that accepted the judged change's trailers, where one did.
    pub approval: Option<&'a Approval>,
}

/// The findings as a comment.
pub fn render(findings: &[Finding], page: &Page) -> String {
    let mut body = format!("{MARKER}\n\n");
    if findings.is_empty() {
        body.push_str("weeder found nothing to report in this pull request.\n");
        return ended(body, page);
    }

    let mut ordered: Vec<&Finding> = findings.iter().collect();
    ordered.sort_by(|left, right| reading(left).cmp(&reading(right)));

    let standings: Vec<Standing> = ordered
        .iter()
        .map(|finding| standing(finding, page))
        .collect();
    let errors = ordered
        .iter()
        .filter(|finding| finding.level == Level::Block)
        .count();
    let accepted = standings
        .iter()
        .filter(|standing| matches!(standing, Standing::Accepted(..)))
        .count();
    let warnings = ordered
        .iter()
        .filter(|finding| finding.level == Level::Warn)
        .count();
    let notes = ordered
        .iter()
        .zip(&standings)
        .filter(|(finding, standing)| {
            finding.level == Level::Note && !matches!(standing, Standing::Accepted(..))
        })
        .count();
    let counts: Vec<String> = [
        (errors, "error"),
        (accepted, "accepted finding"),
        (warnings, "warning"),
        (notes, "note"),
    ]
    .iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, word)| counted(*count, word))
    .collect();
    body.push_str(&format!("weeder found {}. ", listed(&counts)));
    body.push_str(if errors > 0 {
        "An error blocks this pull request, and each finding below says what to do next.\n"
    } else if accepted > 0 {
        "Nothing here blocks this pull request: a reviewer with write access accepted what would have.\n"
    } else {
        "Nothing here blocks this pull request; each finding below is for whoever reviews it.\n"
    });

    for (finding, standing) in ordered.iter().zip(&standings) {
        body.push('\n');
        body.push_str(&item(finding, standing, page));
    }
    ended(body, page)
}

/// The comment for a run that judged nothing, with the reason it could not.
pub fn could_not_judge(reason: &str, page: &Page) -> String {
    let mut body = format!("{MARKER}\n\n");
    body.push_str("weeder could not judge this pull request, so nothing in it was cleared. ");
    body.push_str(&paragraph(reason));
    body.push('\n');
    ended(body, page)
}

/// The order a person reads in: blocks, then warnings, then notes, and inside
/// a level by path, line, rule and the words themselves, so two findings that
/// tie on everything are the same bytes and their order cannot be read.
fn reading(finding: &Finding) -> (u8, &str, u32, &str, [&str; 3]) {
    (
        rank(sarif_level(finding.level)),
        finding.path.as_str(),
        line(finding).map_or(0, |(start, _)| start),
        finding.rule.as_str(),
        parts(finding),
    )
}

/// What became of the allowance a finding carries, if it carries one.
enum Standing<'a> {
    /// Nothing was written to allow it.
    Open,
    /// An allowance took its level off, as it does where no `--strict` asks
    /// otherwise.
    Allowed(&'a Suppression),
    /// A trailer took its level off because a reviewer with write access
    /// approved the judged commit.
    Accepted(&'a Suppression, &'a Approval),
    /// An allowance was written and this run did not honour it.
    Waiting(&'a Suppression),
}

fn standing<'a>(finding: &'a Finding, page: &Page<'a>) -> Standing<'a> {
    let Some(suppression) = finding.suppressed.as_ref() else {
        return Standing::Open;
    };
    // An allowance that was honoured leaves the finding a note; one that was
    // not gives it back the level its rule carries.
    if finding.level != Level::Note {
        return Standing::Waiting(suppression);
    }
    match page.approval {
        Some(approval) if suppression.source == SuppressionSource::CommitTrailer => {
            Standing::Accepted(suppression, approval)
        }
        _ => Standing::Allowed(suppression),
    }
}

/// One finding as an item of the list: its lines joined by hard breaks.
fn item(finding: &Finding, standing: &Standing, page: &Page) -> String {
    let rule = catalogue::rules()
        .iter()
        .find(|known| known.id == finding.rule);
    let shown = match standing {
        Standing::Open | Standing::Waiting(_) => word(finding.level).to_string(),
        Standing::Allowed(suppression) => format!("{}, allowed", original(finding, suppression)),
        Standing::Accepted(suppression, _) => {
            format!("{}, accepted", original(finding, suppression))
        }
    };
    let headline = match rule {
        Some(rule) => format!(
            "**{}** · {shown} · [{}]({})",
            text(rule.short_description),
            code(rule.id),
            help_uri(&docs(page), rule.id),
        ),
        // The ids the catalogue does not hold are weeder's notes about the run
        // itself, the files it set aside or could not read: no rule, no
        // section, and no judgement to call wrong.
        None => format!("**A note on this run** · {shown} · {}", code(&finding.rule)),
    };

    let mut lines = vec![headline, place(finding, page.links)];
    let parts = parts(finding);
    // A what that is only the rule's short description has been said by the
    // headline, and saying it again on the next line reads as a stutter.
    let repeats = |part: &str| rule.is_some_and(|rule| said_alike(part, rule.short_description));
    lines.extend(
        parts
            .iter()
            .enumerate()
            .filter(|(at, part)| !part.is_empty() && !(*at == 0 && repeats(part)))
            .map(|(_, part)| capital(paragraph(part))),
    );
    lines.extend(allowance(finding, standing));
    if rule.is_some() {
        lines.push(format!(
            "[Wrong? Report it]({})",
            report(page, &finding.rule)
        ));
    }
    format!("- {}\n", lines.join("\\\n  "))
}

/// Whether two sentences say the same words, whatever their case and their
/// closing full stop.
fn said_alike(left: &str, right: &str) -> bool {
    let bare = |text: &str| text.trim().trim_end_matches('.').to_lowercase();
    bare(left) == bare(right)
}

/// The level an allowed finding would have reported at.
fn original(finding: &Finding, suppression: &Suppression) -> &'static str {
    word(suppression.original_level.unwrap_or(finding.level))
}

fn word(level: Level) -> &'static str {
    sarif_level(level).word()
}

/// What was said to allow a finding, and what letting a block through takes.
fn allowance(finding: &Finding, standing: &Standing) -> Vec<String> {
    let trailer = format!("{TRAILER} {} <reason>", finding.rule);
    let ask = format!(
        "To let it through, write why in a {} trailer on a commit of this pull request and ask a reviewer with write access to approve its newest commit.",
        code(&trailer)
    );
    match standing {
        Standing::Open if finding.level == Level::Block => vec![ask],
        Standing::Open => Vec::new(),
        Standing::Accepted(suppression, approval) => {
            let names: Vec<String> = approval.reviewers.iter().map(|name| code(name)).collect();
            vec![format!(
                "Accepted by {} at {}, for the reason the trailer gives: {}",
                listed(&names),
                approval.commit,
                paragraph(&suppression.reason)
            )]
        }
        Standing::Allowed(suppression) => vec![format!(
            "Allowed by {}: {}",
            source(suppression),
            paragraph(&suppression.reason)
        )],
        Standing::Waiting(suppression) => match suppression.source {
            // A warning stops nothing, so its trailer waits on no one.
            SuppressionSource::CommitTrailer if finding.level != Level::Block => vec![format!(
                "A {} trailer gives its reason, {}.",
                code(TRAILER),
                quoted(&suppression.reason)
            )],
            SuppressionSource::CommitTrailer => vec![format!(
                "A {} trailer gives its reason, {}, and it lets this through once a reviewer with write access approves the newest commit.",
                code(TRAILER),
                quoted(&suppression.reason)
            )],
            SuppressionSource::InlineComment => {
                let mut said = vec![format!(
                    "A {} marker on the line gives its reason, {}, and a marker is the change's own word, so it never lets a finding through here.",
                    code(LINE_MARKER),
                    quoted(&suppression.reason)
                )];
                if finding.level == Level::Block {
                    said.push(ask);
                }
                said
            }
        },
    }
}

fn source(suppression: &Suppression) -> String {
    match suppression.source {
        SuppressionSource::CommitTrailer => format!("a {} trailer", code(TRAILER)),
        SuppressionSource::InlineComment => format!("a {} marker on the line", code(LINE_MARKER)),
    }
}

/// A reason someone wrote, set off in quotation marks.
fn quoted(reason: &str) -> String {
    format!("\u{201c}{}\u{201d}", paragraph(reason))
}

/// Where weeder's own documentation sits for the version that judged.
fn docs(page: &Page) -> String {
    format!("{}/blob/v{}", page.home.trim_end_matches('/'), page.version)
}

/// A new issue on weeder's own repository from the false-positive form, which
/// the link fills with the rule and the version and nothing else: the change
/// judged may sit in a private repository, and the tracker is public.
fn report(page: &Page, rule: &str) -> String {
    format!(
        "{}/issues/new?template={REPORT_TEMPLATE}&title={}&rule={}&version={}",
        page.home.trim_end_matches('/'),
        encoded(&format!("False positive in {rule}")),
        encoded(rule),
        encoded(page.version),
    )
}

/// The body with the command that judges the change the same way, last.
fn ended(mut body: String, page: &Page) -> String {
    body.push_str(&format!(
        "\nTo see this verdict on your own machine, run {}.\n",
        code(page.command)
    ));
    body
}

/// A line opening with a capital. weeder's own name is lowercase wherever it
/// stands, as its voice says, so a line that opens with it is left as it is.
fn capital(line: String) -> String {
    let opens_with_the_name = line
        .strip_prefix("weeder")
        .is_some_and(|rest| !rest.starts_with(|next: char| next.is_ascii_alphanumeric()));
    let mut characters = line.chars();
    match characters.next() {
        Some(first) if first.is_ascii_lowercase() && !opens_with_the_name => {
            format!("{}{}", first.to_ascii_uppercase(), characters.as_str())
        }
        _ => line,
    }
}

/// The lines a finding names, counted from one, or `None` where it names a
/// whole file.
fn line(finding: &Finding) -> Option<(u32, u32)> {
    finding
        .region
        .as_ref()
        .filter(|region| region.start_line >= 1)
        .map(|region| (region.start_line, region.end_line))
}

/// `a`, `a and b`, `a, b and c`.
fn listed(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The place a finding names, shown as written and linked to its line where
/// the caller said where the files are.
fn place(finding: &Finding, links: Option<&str>) -> String {
    let path = &finding.path;
    let region = line(finding);
    let shown = match region {
        Some((start, _)) => format!("{path}:{start}"),
        None => path.clone(),
    };
    // A note about the change as a whole names the repository as `.`, and a
    // blob address has no file there to show, so it links nowhere.
    let Some(base) = links
        .map(|base| base.trim_end_matches('/'))
        .filter(|_| path != ".")
    else {
        return code(&shown);
    };
    let fragment = match region {
        Some((start, end)) if end > start => format!("#L{start}-L{end}"),
        Some((start, _)) => format!("#L{start}"),
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
