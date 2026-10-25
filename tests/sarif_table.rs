use weeder::core::sarif::render_table;
use weeder::core::{Finding, Level, Message, Region, Suppression, SuppressionSource};

fn finding(rule: &str, level: Level, path: &str, start_line: u32, what: &str) -> Finding {
    Finding {
        rule: rule.to_string(),
        level,
        path: path.to_string(),
        region: Some(Region {
            start_line,
            end_line: start_line,
        }),
        message: Message {
            what: what.to_string(),
            why: "the suite no longer asks the question it used to ask.".to_string(),
            next: "restore it, or record why it went.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

fn suppress(mut finding: Finding) -> Finding {
    finding.suppressed = Some(Suppression {
        rule: finding.rule.clone(),
        reason: "the upstream suite is quarantined".to_string(),
        path: Some(finding.path.clone()),
        line: finding.region.as_ref().map(|region| region.start_line),
        source: SuppressionSource::InlineComment,
        original_level: Some(finding.level),
    });
    finding.level = Level::Note;
    finding
}

fn columns(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

/// The rows of a table: the lines that name a finding, rather than the lines
/// indented under one or the count that ends it.
fn rows(table: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = table.lines().collect();
    lines.pop();
    lines
        .into_iter()
        .filter(|line| !line.starts_with(' '))
        .collect()
}

#[test]
fn table_carries_the_what_on_the_row_and_the_why_and_next_under_it() {
    let findings = vec![
        finding(
            "T1",
            Level::Block,
            "tests/parser.test.ts",
            12,
            "a test case disappeared from a changed test file.",
        ),
        finding(
            "S3",
            Level::Warn,
            "src/parser.rs",
            4,
            "a debug leftover reached production code.",
        ),
    ];
    let table = render_table(&findings);
    let lines: Vec<&str> = table.lines().collect();
    assert_eq!(
        lines.len(),
        7,
        "a row, its why and its next per finding, then the count:\n{table}"
    );

    assert_eq!(
        columns(lines[0])[..3],
        ["error", "T1", "tests/parser.test.ts:12"]
    );
    assert!(lines[0].ends_with("a test case disappeared from a changed test file."));
    assert!(
        !lines[0].contains("the suite no longer asks"),
        "the row carries the what, and the why sits under it:\n{table}"
    );
    let column = lines[0].find("T1").expect("the row names its rule");
    assert_eq!(
        lines[1],
        format!(
            "{}why   the suite no longer asks the question it used to ask.",
            " ".repeat(column)
        ),
        "the why is indented under the row, beneath the rule:\n{table}"
    );
    assert_eq!(
        lines[2],
        format!(
            "{}next  restore it, or record why it went.",
            " ".repeat(column)
        ),
        "the next action follows the why:\n{table}"
    );

    assert_eq!(columns(lines[3])[..3], ["warning", "S3", "src/parser.rs:4"]);
    assert!(lines[3].ends_with("a debug leftover reached production code."));
    assert!(lines[4].trim_start().starts_with("why "));
    assert!(lines[5].trim_start().starts_with("next "));
}

#[test]
fn a_long_why_wraps_under_its_row_rather_than_running_on() {
    let mut long = finding(
        "T2",
        Level::Block,
        "tests/test_format.py",
        3,
        "2 assertions went out of this file.",
    );
    long.message.why = "a case that checks nothing passes whatever the code does, so the suite reports green over behaviour held by nobody, and the next change to that behaviour lands unwatched.".to_string();
    let table = render_table(&[long.clone()]);
    let lines: Vec<&str> = table.lines().collect();
    let text = lines[0].find("T2").expect("the row names its rule") + "why   ".len();
    let why: Vec<&str> = lines[1..]
        .iter()
        .take_while(|line| !line.trim_start().starts_with("next"))
        .copied()
        .collect();
    assert!(
        why.len() > 1,
        "a why longer than a line wraps onto more than one:\n{table}"
    );
    assert!(
        why.iter().all(|line| line.chars().count() <= text + 72),
        "no wrapped line runs past the width:\n{table}"
    );
    assert!(
        why[1..]
            .iter()
            .all(|line| line[..text].trim().is_empty() && !line[text..].starts_with(' ')),
        "each continuation starts where the why's text does:\n{table}"
    );
    let joined = std::iter::once(&why[0][text..])
        .chain(why[1..].iter().map(|line| &line[text..]))
        .collect::<Vec<&str>>()
        .join(" ");
    assert_eq!(joined, long.message.why, "wrapping loses no word");
}

#[test]
fn table_orders_findings_by_level_then_path_then_line() {
    let findings = vec![
        finding("S3", Level::Warn, "src/b.rs", 5, "a debug leftover."),
        suppress(finding("T3", Level::Block, "tests/a.test.ts", 9, "a skip.")),
        finding("S1", Level::Block, "src/b.rs", 40, "a stub."),
        finding("T4", Level::Warn, "src/a.rs", 90, "a widened tolerance."),
        finding("S1", Level::Block, "src/b.rs", 7, "a stub."),
        finding("G1", Level::Block, "src/a.rs", 3, "a conflict marker."),
    ];
    let table = render_table(&findings);
    let order: Vec<String> = rows(&table)
        .into_iter()
        .map(|line| {
            let cells = columns(line);
            format!("{} {}", cells[0], cells[2])
        })
        .collect();
    assert_eq!(
        order,
        vec![
            "error src/a.rs:3",
            "error src/b.rs:7",
            "error src/b.rs:40",
            "warning src/a.rs:90",
            "warning src/b.rs:5",
            "note tests/a.test.ts:9",
        ]
    );
}

#[test]
fn table_ends_with_a_count_per_level() {
    let findings = vec![
        finding("T1", Level::Block, "tests/a.test.ts", 4, "a test went."),
        finding("S1", Level::Block, "src/a.rs", 4, "a stub."),
        finding("S3", Level::Warn, "src/a.rs", 9, "a debug leftover."),
        suppress(finding("T3", Level::Block, "tests/a.test.ts", 2, "a skip.")),
        suppress(finding("T3", Level::Block, "tests/b.test.ts", 2, "a skip.")),
        suppress(finding("T3", Level::Block, "tests/c.test.ts", 2, "a skip.")),
    ];
    let table = render_table(&findings);
    assert_eq!(
        table.lines().last().expect("the table ends with a count"),
        "2 errors, 1 warning, 3 notes"
    );
}

#[test]
fn an_empty_run_renders_the_count_line_and_nothing_else() {
    let table = render_table(&[]);
    assert_eq!(
        table.lines().collect::<Vec<&str>>(),
        vec!["0 errors, 0 warnings, 0 notes"]
    );
}
