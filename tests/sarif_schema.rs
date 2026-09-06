use std::fs;
use std::path::Path;

use jsonschema::Validator;
use weed::core::sarif::{render, to_json, Context, Log};
use weed::core::{Finding, Fix, Level, Message, Region, Suppression, SuppressionSource};

fn validator() -> Validator {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/sarif-schema-2.1.0.json");
    let source = fs::read_to_string(&path).expect("the vendored schema should be readable");
    let schema = serde_json::from_str(&source).expect("the vendored schema should be json");
    jsonschema::validator_for(&schema).expect("the vendored schema should compile")
}

/// Validation reads the json weed writes, not an in-memory value, so the text a
/// consumer receives is the text under test.
fn assert_validates(log: &Log) {
    let written = to_json(log);
    let value: serde_json::Value =
        serde_json::from_str(&written).expect("weed should write parseable json");
    let validator = validator();
    let complaints: Vec<String> = validator
        .iter_errors(&value)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(
        complaints.is_empty(),
        "log does not validate against the vendored schema:\n{}\n{}",
        complaints.join("\n"),
        written
    );
}

fn context() -> Context {
    Context::new("0.1.0").with_docs_base("file:///srv/weed/")
}

fn finding(rule: &str, level: Level, path: &str, start_line: u32) -> Finding {
    Finding {
        rule: rule.to_string(),
        level,
        path: path.to_string(),
        region: Some(Region {
            start_line,
            end_line: start_line,
        }),
        message: Message {
            what: format!("{rule} fired on {path}."),
            why: "the change is weaker than what it replaced.".to_string(),
            next: "restore it or record a reason.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

#[test]
fn empty_log_validates_against_the_vendored_schema() {
    assert_validates(&render(&[], &context()));
}

#[test]
fn single_finding_log_validates_against_the_vendored_schema() {
    let findings = vec![finding("T1", Level::Block, "tests/parser.test.ts", 12)];
    assert_validates(&render(&findings, &context()));
}

#[test]
fn many_findings_across_levels_validate_against_the_vendored_schema() {
    let mut findings = vec![
        finding("T1", Level::Block, "tests/parser.test.ts", 12),
        finding("S1", Level::Block, "src/parser.rs", 40),
        finding("T4", Level::Warn, "tests/timeout.test.ts", 7),
        finding("S3", Level::Warn, "src/main.go", 3),
        finding("G2", Level::Note, "assets/logo.png", 1),
    ];
    findings[2].region = Some(Region {
        start_line: 7,
        end_line: 19,
    });
    findings[4].region = None;
    assert_validates(&render(&findings, &context()));
}

#[test]
fn finding_with_a_fix_validates_against_the_vendored_schema() {
    let mut findings = vec![finding("T3", Level::Block, "tests/parser.test.ts", 12)];
    findings[0].fix = Some(Fix {
        description: "remove the skip marker.".to_string(),
        replacement: Some("it(\"parses a rename\", () => {})".to_string()),
    });
    let mut removal = finding("G1", Level::Block, "src/parser.rs", 88);
    removal.fix = Some(Fix {
        description: "delete the conflict marker.".to_string(),
        replacement: None,
    });
    findings.push(removal);
    assert_validates(&render(&findings, &context()));
}

#[test]
fn suppressed_finding_validates_against_the_vendored_schema() {
    let mut findings = vec![finding("T3", Level::Note, "tests/parser.test.ts", 12)];
    findings[0].suppressed = Some(Suppression {
        rule: "T3".to_string(),
        reason: "the upstream suite is quarantined until it is rewritten".to_string(),
        path: Some("tests/parser.test.ts".to_string()),
        line: Some(11),
        source: SuppressionSource::InlineComment,
        original_level: Some(Level::Block),
    });
    assert_validates(&render(&findings, &context()));
}

#[test]
fn could_not_run_log_validates_against_the_vendored_schema() {
    let context = context().could_not_run("the diff names a path weed cannot parse.");
    assert_validates(&render(&[], &context));
}
