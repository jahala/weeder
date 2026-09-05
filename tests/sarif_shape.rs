use serde_json::Value;

use weed::core::catalogue::{self, Face};
use weed::core::sarif::{render, Context, SARIF_VERSION, SCHEMA_URI};
use weed::core::{Finding, Fix, Level, Message, Region, Suppression, SuppressionSource};

fn context() -> Context {
    Context::new("0.1.0").with_docs_base("file:///srv/weed/")
}

fn json(findings: &[Finding], context: &Context) -> Value {
    serde_json::to_value(render(findings, context)).expect("a log should serialize")
}

fn finding(rule: &str, level: Level, path: &str, region: Option<Region>) -> Finding {
    Finding {
        rule: rule.to_string(),
        level,
        path: path.to_string(),
        region,
        message: Message {
            what: "a test case disappeared from a changed test file.".to_string(),
            why: "the suite still passes because it no longer asks the question.".to_string(),
            next: "restore the case, or record why it went with a reason.".to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

fn region(start_line: u32, end_line: u32) -> Option<Region> {
    Some(Region {
        start_line,
        end_line,
    })
}

#[test]
fn log_holds_one_run_with_the_schema_uri_and_version() {
    let log = json(
        &[finding("T1", Level::Block, "tests/a.test.ts", region(4, 4))],
        &context(),
    );
    assert_eq!(log["$schema"], Value::from(SCHEMA_URI));
    assert_eq!(log["version"], Value::from(SARIF_VERSION));
    assert_eq!(
        log["runs"].as_array().expect("runs is an array").len(),
        1,
        "one run per invocation"
    );
    assert_eq!(
        log["runs"][0]["tool"]["driver"]["name"],
        Value::from("weed")
    );
    assert_eq!(
        log["runs"][0]["tool"]["driver"]["version"],
        Value::from("0.1.0")
    );
}

#[test]
fn result_carries_its_rule_id_and_the_index_of_that_rule_in_the_tool_component() {
    let log = json(
        &[
            finding("S1", Level::Block, "src/parser.rs", region(40, 40)),
            finding("R2", Level::Warn, "src/parser.rs", region(9, 9)),
        ],
        &context(),
    );
    let rules = log["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("the driver lists rules")
        .clone();
    for (result, expected) in log["runs"][0]["results"]
        .as_array()
        .expect("results is an array")
        .iter()
        .zip(["S1", "R2"])
    {
        assert_eq!(result["ruleId"], Value::from(expected));
        let index = result["ruleIndex"]
            .as_u64()
            .expect("a catalogue rule has an index") as usize;
        assert_eq!(rules[index]["id"], Value::from(expected));
    }
}

#[test]
fn levels_map_block_to_error_warn_to_warning_and_a_suppressed_finding_to_note() {
    let mut suppressed = finding("T3", Level::Block, "tests/a.test.ts", region(2, 2));
    suppressed.suppressed = Some(Suppression {
        rule: "T3".to_string(),
        reason: "the upstream suite is quarantined".to_string(),
        path: Some("tests/a.test.ts".to_string()),
        line: Some(1),
        source: SuppressionSource::InlineComment,
        original_level: Some(Level::Block),
    });
    let findings = vec![
        finding("T1", Level::Block, "tests/a.test.ts", region(4, 4)),
        finding("T4", Level::Warn, "tests/a.test.ts", region(6, 6)),
        suppressed,
    ];
    let log = json(&findings, &context());
    let results = log["runs"][0]["results"]
        .as_array()
        .expect("results is an array");
    let levels: Vec<&str> = results
        .iter()
        .map(|result| result["level"].as_str().expect("a result carries a level"))
        .collect();
    assert_eq!(levels, vec!["error", "warning", "note"]);
}

#[test]
fn location_is_a_repo_relative_uri_with_a_region() {
    let log = json(
        &[finding("T1", Level::Block, "tests/a.test.ts", region(4, 9))],
        &context(),
    );
    let location = &log["runs"][0]["results"][0]["locations"][0]["physicalLocation"];
    assert_eq!(
        location["artifactLocation"]["uri"],
        Value::from("tests/a.test.ts")
    );
    assert_eq!(location["region"]["startLine"], Value::from(4));
    assert_eq!(location["region"]["endLine"], Value::from(9));

    let single = json(
        &[finding("T1", Level::Block, "tests/a.test.ts", region(4, 4))],
        &context(),
    );
    let region = &single["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
    assert_eq!(region["startLine"], Value::from(4));
    assert!(
        region.get("endLine").is_none(),
        "a one-line finding carries startLine alone"
    );
}

#[test]
fn message_reads_what_then_why_then_next() {
    let one = finding("T1", Level::Block, "tests/a.test.ts", region(4, 4));
    let log = json(std::slice::from_ref(&one), &context());
    let text = log["runs"][0]["results"][0]["message"]["text"]
        .as_str()
        .expect("a result carries a message")
        .to_string();
    assert_eq!(
        text,
        format!(
            "{} {} {}",
            one.message.what, one.message.why, one.message.next
        )
    );
}

#[test]
fn a_mechanical_fix_becomes_a_fixes_entry_over_the_finding_s_region() {
    let mut replaced = finding("T3", Level::Block, "tests/a.test.ts", region(2, 2));
    replaced.fix = Some(Fix {
        description: "remove the skip marker.".to_string(),
        replacement: Some("it(\"parses\", () => {})".to_string()),
    });
    let mut removed = finding("G1", Level::Block, "src/parser.rs", region(88, 90));
    removed.fix = Some(Fix {
        description: "delete the conflict marker.".to_string(),
        replacement: None,
    });
    let plain = finding("T1", Level::Block, "tests/b.test.ts", region(1, 1));
    let log = json(&[replaced, removed, plain], &context());
    let results = log["runs"][0]["results"]
        .as_array()
        .expect("results is an array");

    let change = &results[0]["fixes"][0]["artifactChanges"][0];
    assert_eq!(
        results[0]["fixes"][0]["description"]["text"],
        Value::from("remove the skip marker.")
    );
    assert_eq!(
        change["artifactLocation"]["uri"],
        Value::from("tests/a.test.ts")
    );
    assert_eq!(
        change["replacements"][0]["deletedRegion"]["startLine"],
        Value::from(2)
    );
    assert_eq!(
        change["replacements"][0]["insertedContent"]["text"],
        Value::from("it(\"parses\", () => {})")
    );

    let removal = &results[1]["fixes"][0]["artifactChanges"][0]["replacements"][0];
    assert_eq!(removal["deletedRegion"]["startLine"], Value::from(88));
    assert_eq!(removal["deletedRegion"]["endLine"], Value::from(90));
    assert!(
        removal.get("insertedContent").is_none(),
        "a removal inserts nothing"
    );

    assert!(
        results[2].get("fixes").is_none(),
        "a finding with no mechanical fix carries no fixes entry"
    );
}

#[test]
fn a_suppressed_result_carries_the_reason_as_an_in_source_justification() {
    let mut suppressed = finding("T3", Level::Note, "tests/a.test.ts", region(2, 2));
    suppressed.suppressed = Some(Suppression {
        rule: "T3".to_string(),
        reason: "the upstream suite is quarantined until it is rewritten".to_string(),
        path: Some("tests/a.test.ts".to_string()),
        line: Some(1),
        source: SuppressionSource::CommitTrailer,
        original_level: Some(Level::Block),
    });
    let plain = finding("T1", Level::Block, "tests/b.test.ts", region(1, 1));
    let log = json(&[suppressed, plain], &context());
    let results = log["runs"][0]["results"]
        .as_array()
        .expect("results is an array");
    assert_eq!(
        results[0]["suppressions"][0]["kind"],
        Value::from("inSource")
    );
    assert_eq!(
        results[0]["suppressions"][0]["justification"],
        Value::from("the upstream suite is quarantined until it is rewritten")
    );
    assert!(
        results[1].get("suppressions").is_none(),
        "an unsuppressed result carries no suppressions"
    );
}

#[test]
fn tool_component_lists_every_catalogue_rule_with_its_default_level() {
    let log = json(&[], &context());
    let rules = log["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("the driver lists rules");
    assert_eq!(rules.len(), catalogue::rules().len());
    assert!(!rules.is_empty(), "the catalogue is not empty");

    for (rendered, rule) in rules.iter().zip(catalogue::rules()) {
        assert_eq!(rendered["id"], Value::from(rule.id));
        assert_eq!(
            rendered["shortDescription"]["text"],
            Value::from(rule.short_description)
        );
        assert_eq!(
            rendered["fullDescription"]["text"],
            Value::from(rule.full_description)
        );
        let expected = match rule.default_level {
            Level::Block => "error",
            Level::Warn => "warning",
            Level::Note => "note",
        };
        assert_eq!(
            rendered["defaultConfiguration"]["level"],
            Value::from(expected),
            "{} carries its default level",
            rule.id
        );
        assert_eq!(
            rendered["helpUri"],
            Value::from(format!("file:///srv/weed/docs/rules.md#{}", rule.id))
        );
    }

    assert!(
        catalogue::rules()
            .iter()
            .any(|rule| rule.face == Face::Check),
        "the catalogue holds check rules"
    );
    assert!(
        catalogue::rules()
            .iter()
            .any(|rule| rule.face == Face::Scan),
        "the catalogue holds scan rules"
    );
}

#[test]
fn the_invocation_reports_the_exit_code_and_the_reason_weed_could_not_run() {
    let clean = json(&[], &context());
    assert_eq!(
        clean["runs"][0]["invocations"][0]["executionSuccessful"],
        Value::from(true)
    );
    assert_eq!(
        clean["runs"][0]["invocations"][0]["exitCode"],
        Value::from(0)
    );

    let blocked = json(
        &[finding("T1", Level::Block, "tests/a.test.ts", region(4, 4))],
        &context(),
    );
    assert_eq!(
        blocked["runs"][0]["invocations"][0]["exitCode"],
        Value::from(2)
    );

    let warned = json(
        &[finding("T4", Level::Warn, "tests/a.test.ts", region(4, 4))],
        &context(),
    );
    assert_eq!(
        warned["runs"][0]["invocations"][0]["exitCode"],
        Value::from(0)
    );

    let stopped = json(
        &[],
        &context().could_not_run("the diff names a path weed cannot parse."),
    );
    let invocation = &stopped["runs"][0]["invocations"][0];
    assert_eq!(invocation["executionSuccessful"], Value::from(false));
    assert_eq!(invocation["exitCode"], Value::from(3));
    assert_eq!(
        invocation["toolExecutionNotifications"][0]["message"]["text"],
        Value::from("the diff names a path weed cannot parse.")
    );
}
