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

#[test]
fn table_carries_level_rule_path_line_and_the_what_on_one_line_per_finding() {
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
    assert_eq!(lines.len(), 3, "one line per finding, then the count");

    assert_eq!(
        columns(lines[0])[..3],
        ["error", "T1", "tests/parser.test.ts:12"]
    );
    assert!(lines[0].ends_with("a test case disappeared from a changed test file."));
    assert!(
        !lines[0].contains("the suite no longer asks"),
        "the table carries the what alone"
    );

    assert_eq!(columns(lines[1])[..3], ["warning", "S3", "src/parser.rs:4"]);
    assert!(lines[1].ends_with("a debug leftover reached production code."));
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
    let order: Vec<String> = table
        .lines()
        .take(6)
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
