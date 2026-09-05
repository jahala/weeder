use serde::Serialize;

use crate::core::catalogue::{self, Rule};
use crate::core::finding::{Finding, Level};

/// The SARIF version every log declares.
pub const SARIF_VERSION: &str = "2.1.0";
/// The `$schema` a consumer validates against; the same document is vendored at
/// `schemas/sarif-schema-2.1.0.json` so validation never needs the network.
pub const SCHEMA_URI: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json";
/// The tool component's name.
pub const TOOL_NAME: &str = "weed";
/// Where a rule is documented, relative to the repository root.
pub const RULES_DOC: &str = "docs/rules.md";

/// Clean, or warnings only.
pub const EXIT_CLEAN: i32 = 0;
/// At least one block-level result.
pub const EXIT_BLOCKED: i32 = 2;
/// weed could not run, so the gate fails closed.
pub const EXIT_COULD_NOT_RUN: i32 = 3;

/// Whether the run reached a judgement at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Ran,
    CouldNotRun { reason: String },
}

/// What a face knows and the core does not: the binary's version, the rules in
/// play, where they are documented, and whether the run got that far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub tool_version: String,
    pub catalogue: Vec<Rule>,
    /// An absolute URI that `docs/rules.md#<id>` resolves against. SARIF wants
    /// an absolute `helpUri`, so without a base weed writes no `helpUri` at all
    /// rather than a relative one no consumer would accept.
    pub docs_base: Option<String>,
    pub outcome: Outcome,
}

impl Context {
    /// A run of the whole catalogue that reached a judgement.
    pub fn new(tool_version: impl Into<String>) -> Self {
        Self {
            tool_version: tool_version.into(),
            catalogue: catalogue::rules().to_vec(),
            docs_base: None,
            outcome: Outcome::Ran,
        }
    }

    pub fn with_docs_base(mut self, base: impl Into<String>) -> Self {
        self.docs_base = Some(base.into());
        self
    }

    pub fn with_catalogue(mut self, catalogue: Vec<Rule>) -> Self {
        self.catalogue = catalogue;
        self
    }

    pub fn could_not_run(mut self, reason: impl Into<String>) -> Self {
        self.outcome = Outcome::CouldNotRun {
            reason: reason.into(),
        };
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SarifLevel {
    Error,
    Warning,
    Note,
}

impl SarifLevel {
    fn word(self) -> &'static str {
        match self {
            SarifLevel::Error => "error",
            SarifLevel::Warning => "warning",
            SarifLevel::Note => "note",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Text {
    pub text: String,
}

impl Text {
    fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Log {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Run {
    pub tool: Tool,
    pub invocations: Vec<Invocation>,
    pub results: Vec<SarifResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tool {
    pub driver: Driver,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Driver {
    pub name: String,
    pub version: String,
    pub rules: Vec<ReportingDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingDescriptor {
    pub id: String,
    pub short_description: Text,
    pub full_description: Text,
    pub default_configuration: ReportingConfiguration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportingConfiguration {
    pub level: SarifLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invocation {
    pub execution_successful: bool,
    pub exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_execution_notifications: Option<Vec<Notification>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Notification {
    pub level: SarifLevel,
    pub message: Text,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifResult {
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_index: Option<usize>,
    pub level: SarifLevel,
    pub message: Text,
    pub locations: Vec<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixes: Option<Vec<SarifFix>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppressions: Option<Vec<SarifSuppression>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub physical_location: PhysicalLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalLocation {
    pub artifact_location: ArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SarifRegion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifRegion {
    pub start_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifFix {
    pub description: Text,
    pub artifact_changes: Vec<ArtifactChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactChange {
    pub artifact_location: ArtifactLocation,
    pub replacements: Vec<Replacement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Replacement {
    pub deleted_region: SarifRegion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inserted_content: Option<Text>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifSuppression {
    pub kind: String,
    pub justification: String,
}

/// Findings and what the face knows, as one SARIF run.
pub fn render(findings: &[Finding], context: &Context) -> Log {
    let rules = context
        .catalogue
        .iter()
        .map(|rule| descriptor(rule, context.docs_base.as_deref()))
        .collect();
    let mut results: Vec<SarifResult> = findings
        .iter()
        .map(|finding| result(finding, &context.catalogue))
        .collect();
    results.sort_by(|left, right| sort_key(left).cmp(&sort_key(right)));
    let invocation = match &context.outcome {
        Outcome::Ran => Invocation {
            execution_successful: true,
            exit_code: if results.iter().any(|r| r.level == SarifLevel::Error) {
                EXIT_BLOCKED
            } else {
                EXIT_CLEAN
            },
            tool_execution_notifications: None,
        },
        Outcome::CouldNotRun { reason } => Invocation {
            execution_successful: false,
            exit_code: EXIT_COULD_NOT_RUN,
            tool_execution_notifications: Some(vec![Notification {
                level: SarifLevel::Error,
                message: Text::new(reason),
            }]),
        },
    };

    Log {
        schema: SCHEMA_URI.to_string(),
        version: SARIF_VERSION.to_string(),
        runs: vec![Run {
            tool: Tool {
                driver: Driver {
                    name: TOOL_NAME.to_string(),
                    version: context.tool_version.clone(),
                    rules,
                },
            },
            invocations: vec![invocation],
            results,
        }],
    }
}

/// The order law: file, then line, then rule id, and the message where a file,
/// a line and a rule still name two results. Fixing the order here, once, is
/// what lets weed promise the same bytes on two runs: a detector may report in
/// whatever order suits it, a face may add findings from several passes, and the
/// log still comes out the same. A finding with no region points at the whole
/// file and sorts at line zero, above every line in it. The message closes the
/// order so that no two results can be told apart by position alone — results
/// that tie on all four are the same bytes, so their order cannot be read.
fn sort_key(result: &SarifResult) -> (&str, u32, &str, &str) {
    let place = result
        .locations
        .first()
        .map(|location| &location.physical_location);
    (
        place
            .map(|place| place.artifact_location.uri.as_str())
            .unwrap_or_default(),
        place
            .and_then(|place| place.region.as_ref())
            .map(|region| region.start_line)
            .unwrap_or(0),
        result.rule_id.as_str(),
        result.message.text.as_str(),
    )
}

/// A log as the JSON a consumer reads. Serializing cannot fail: every field is
/// a string, a number, a bool, or a struct of those, so there is no map with a
/// non-string key and no floating-point value to reject.
pub fn to_json(log: &Log) -> String {
    serde_json::to_string_pretty(log).expect("a log holds only values serde_json can write")
}

/// One line per finding for a terminal: level, rule, `path:line`, the what.
/// Block findings first, then warnings, then notes; inside a level by path then
/// line. The last line counts each level.
pub fn render_table(findings: &[Finding]) -> String {
    let mut rows: Vec<Row> = findings.iter().map(row).collect();
    rows.sort_by(|left, right| {
        rank(left.level)
            .cmp(&rank(right.level))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });

    let level_width = rows.iter().map(|row| row.level.word().len()).max();
    let rule_width = rows.iter().map(|row| row.rule.len()).max();
    let place_width = rows.iter().map(|row| row.place.len()).max();

    let mut table = String::new();
    for row in &rows {
        let line = format!(
            "{level:<level_width$}  {rule:<rule_width$}  {place:<place_width$}  {what}",
            level = row.level.word(),
            rule = row.rule,
            place = row.place,
            what = row.what,
            level_width = level_width.unwrap_or_default(),
            rule_width = rule_width.unwrap_or_default(),
            place_width = place_width.unwrap_or_default(),
        );
        table.push_str(line.trim_end());
        table.push('\n');
    }

    let count = |level: SarifLevel| rows.iter().filter(|row| row.level == level).count();
    table.push_str(&format!(
        "{}, {}, {}\n",
        counted(count(SarifLevel::Error), "error"),
        counted(count(SarifLevel::Warning), "warning"),
        counted(count(SarifLevel::Note), "note"),
    ));
    table
}

struct Row {
    level: SarifLevel,
    rule: String,
    path: String,
    line: u32,
    place: String,
    what: String,
}

fn row(finding: &Finding) -> Row {
    let line = region(finding).map(|region| region.start_line).unwrap_or(0);
    let place = if line == 0 {
        finding.path.clone()
    } else {
        format!("{}:{}", finding.path, line)
    };
    Row {
        level: sarif_level(finding.level),
        rule: finding.rule.clone(),
        path: finding.path.clone(),
        line,
        place,
        what: finding.message.what.trim().to_string(),
    }
}

fn counted(count: usize, word: &str) -> String {
    if count == 1 {
        format!("{count} {word}")
    } else {
        format!("{count} {word}s")
    }
}

fn rank(level: SarifLevel) -> u8 {
    match level {
        SarifLevel::Error => 0,
        SarifLevel::Warning => 1,
        SarifLevel::Note => 2,
    }
}

fn descriptor(rule: &Rule, docs_base: Option<&str>) -> ReportingDescriptor {
    ReportingDescriptor {
        id: rule.id.to_string(),
        short_description: Text::new(rule.short_description),
        full_description: Text::new(rule.full_description),
        default_configuration: ReportingConfiguration {
            level: sarif_level(rule.default_level),
        },
        help_uri: docs_base.map(|base| help_uri(base, rule.id)),
    }
}

fn help_uri(docs_base: &str, id: &str) -> String {
    let separator = if docs_base.ends_with('/') { "" } else { "/" };
    format!("{docs_base}{separator}{RULES_DOC}#{id}")
}

fn result(finding: &Finding, catalogue: &[Rule]) -> SarifResult {
    let region = region(finding).map(|region| sarif_region(region.start_line, region.end_line));
    SarifResult {
        rule_id: finding.rule.clone(),
        rule_index: catalogue.iter().position(|rule| rule.id == finding.rule),
        level: sarif_level(finding.level),
        message: Text::new(message(finding)),
        locations: vec![Location {
            physical_location: PhysicalLocation {
                artifact_location: ArtifactLocation {
                    uri: finding.path.clone(),
                },
                region: region.clone(),
            },
        }],
        fixes: finding
            .fix
            .as_ref()
            .zip(region)
            .map(|(fix, region)| vec![sarif_fix(fix, &finding.path, region)]),
        suppressions: finding.suppressed.as_ref().map(|suppression| {
            vec![SarifSuppression {
                kind: "inSource".to_string(),
                justification: suppression.reason.clone(),
            }]
        }),
    }
}

fn sarif_fix(fix: &crate::core::finding::Fix, path: &str, deleted_region: SarifRegion) -> SarifFix {
    SarifFix {
        description: Text::new(&fix.description),
        artifact_changes: vec![ArtifactChange {
            artifact_location: ArtifactLocation {
                uri: path.to_string(),
            },
            replacements: vec![Replacement {
                deleted_region,
                inserted_content: fix.replacement.as_ref().map(Text::new),
            }],
        }],
    }
}

fn sarif_region(start_line: u32, end_line: u32) -> SarifRegion {
    SarifRegion {
        start_line,
        end_line: if end_line > start_line {
            Some(end_line)
        } else {
            None
        },
    }
}

/// SARIF counts lines from one, so a finding whose region starts at line zero
/// carries no region and points at the file instead.
fn region(finding: &Finding) -> Option<&crate::core::finding::Region> {
    finding
        .region
        .as_ref()
        .filter(|region| region.start_line >= 1)
}

fn sarif_level(level: Level) -> SarifLevel {
    match level {
        Level::Block => SarifLevel::Error,
        Level::Warn => SarifLevel::Warning,
        Level::Note => SarifLevel::Note,
    }
}

/// What was found, why it matters, then the next action, in that order.
fn message(finding: &Finding) -> String {
    [
        finding.message.what.trim(),
        finding.message.why.trim(),
        finding.message.next.trim(),
    ]
    .iter()
    .filter(|part| !part.is_empty())
    .copied()
    .collect::<Vec<&str>>()
    .join(" ")
}
