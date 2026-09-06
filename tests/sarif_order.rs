//! The order law. Two runs on the same diff must write the same bytes, and the
//! only way to promise that is to fix the order before the log is built:
//! results by file, then by line, then by rule id. These tests hand `render`
//! findings in orders no caller would ever produce and read the log back.

use weeder::core::sarif::{render, Context};
use weeder::core::{Finding, Level, Message, Region};

fn context() -> Context {
    Context::new("0.1.0")
}

fn finding(rule: &str, path: &str, start_line: u32) -> Finding {
    Finding {
        rule: rule.to_string(),
        level: Level::Warn,
        path: path.to_string(),
        region: (start_line >= 1).then_some(Region {
            start_line,
            end_line: start_line,
        }),
        message: Message {
            what: "something was found.".to_string(),
            why: "it matters.".to_string(),
            next: "do this next.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

/// Every result as the triple the law sorts on. A result with no region reports
/// its line as zero, which is where the law puts a whole-file finding.
fn order(findings: &[Finding]) -> Vec<(String, u32, String)> {
    render(findings, &context()).runs[0]
        .results
        .iter()
        .map(|result| {
            let location = &result.locations[0].physical_location;
            (
                location.artifact_location.uri.clone(),
                location
                    .region
                    .as_ref()
                    .map(|region| region.start_line)
                    .unwrap_or(0),
                result.rule_id.clone(),
            )
        })
        .collect()
}

/// The same findings, handed in every order there is. Any input order the law
/// holds for is one fewer way for two runs to disagree.
fn every_permutation(findings: &[Finding]) -> Vec<Vec<Finding>> {
    if findings.is_empty() {
        return vec![Vec::new()];
    }
    let mut permutations = Vec::new();
    for (index, finding) in findings.iter().enumerate() {
        let mut rest = findings.to_vec();
        rest.remove(index);
        for mut tail in every_permutation(&rest) {
            tail.insert(0, finding.clone());
            permutations.push(tail);
        }
    }
    permutations
}

#[test]
fn results_are_ordered_by_file_then_line_then_rule() {
    let shuffled = [
        finding("T3", "src/parser.ts", 12),
        finding("G1", "src/parser.ts", 12),
        finding("S1", "README.md", 40),
        finding("G1", "src/parser.ts", 3),
        finding("X1", "src/app/main.ts", 200),
    ];

    assert_eq!(
        order(&shuffled),
        vec![
            ("README.md".to_string(), 40, "S1".to_string()),
            ("src/app/main.ts".to_string(), 200, "X1".to_string()),
            ("src/parser.ts".to_string(), 3, "G1".to_string()),
            ("src/parser.ts".to_string(), 12, "G1".to_string()),
            ("src/parser.ts".to_string(), 12, "T3".to_string()),
        ],
        "results come out by file, then line, then rule id"
    );
}

#[test]
fn line_order_is_numeric_and_not_the_order_the_text_would_give() {
    let shuffled = [
        finding("G1", "src/parser.ts", 100),
        finding("G1", "src/parser.ts", 9),
        finding("G1", "src/parser.ts", 21),
    ];

    let lines: Vec<u32> = order(&shuffled)
        .into_iter()
        .map(|(_, line, _)| line)
        .collect();
    assert_eq!(
        lines,
        vec![9, 21, 100],
        "line 9 comes before line 100, which sorting the text would not give"
    );
}

#[test]
fn a_finding_with_no_region_comes_before_the_lines_of_its_file() {
    let shuffled = [
        finding("D2", "src/core/sarif.rs", 8),
        finding("X2", "src/core/sarif.rs", 0),
    ];

    assert_eq!(
        order(&shuffled),
        vec![
            ("src/core/sarif.rs".to_string(), 0, "X2".to_string()),
            ("src/core/sarif.rs".to_string(), 8, "D2".to_string()),
        ],
        "a whole-file finding sorts at line zero, above every line in that file"
    );
}

#[test]
fn every_input_order_of_the_same_findings_writes_the_same_log() {
    let findings = [
        finding("T1", "tests/parse.test.ts", 4),
        finding("G1", "src/parser.ts", 12),
        finding("G1", "src/parser.ts", 3),
        finding("S1", "src/parser.ts", 12),
        finding("X1", ".github/workflows/ci.yml", 1),
    ];

    let permutations = every_permutation(&findings);
    assert_eq!(permutations.len(), 120, "five findings permute 120 ways");

    let first = weeder::core::sarif::to_json(&render(&permutations[0], &context()));
    for permutation in &permutations {
        assert_eq!(
            weeder::core::sarif::to_json(&render(permutation, &context())),
            first,
            "the log must not depend on the order the findings arrived in"
        );
    }
}

/// Two findings a file, a line and a rule cannot tell apart still have to come
/// out the same way round every time, or the order law leaves a gap wide enough
/// for a diff to answer differently on two runs.
#[test]
fn two_findings_alike_in_file_line_and_rule_are_still_ordered() {
    let mut first = finding("G1", "src/parser.ts", 12);
    first.message.what = "a merge conflict marker (<<<<<<<) was added.".to_string();
    let mut second = finding("G1", "src/parser.ts", 12);
    second.message.what = "a merge conflict marker (=======) was added.".to_string();

    let forwards =
        weeder::core::sarif::to_json(&render(&[first.clone(), second.clone()], &context()));
    let backwards = weeder::core::sarif::to_json(&render(&[second, first], &context()));
    assert_eq!(
        forwards, backwards,
        "the message closes the order where file, line and rule tie"
    );
}

#[test]
fn the_table_keeps_its_level_first_order() {
    let findings = [
        finding("N1", "src/a.ts", 2),
        Finding {
            level: Level::Block,
            ..finding("G1", "src/z.ts", 90)
        },
    ];

    let table = weeder::core::sarif::render_table(&findings);
    let first = table.lines().next().expect("the table has a first row");
    assert!(
        first.starts_with("error"),
        "the table sorts by level first, so the block finding leads: {table}"
    );
}
