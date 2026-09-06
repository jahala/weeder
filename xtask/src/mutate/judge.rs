//! The binary, asked. Nothing in the campaign reads weed's core directly: a
//! case is judged by running `weed check` the way a gate runs it, and by
//! reading the SARIF that comes back.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

/// One result weed reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: String,
    pub level: String,
    pub path: String,
    pub line: Option<u32>,
}

/// Run `weed check --base <base> --strict` over the working tree and answer
/// with what it found. An exit of 3 is weed saying it could not run, and that
/// is never a clean case.
pub fn check(
    binary: &Path,
    root: &Path,
    base: &str,
    arguments: &[String],
) -> Result<Vec<Finding>, String> {
    let output = Command::new(binary)
        .current_dir(root)
        .args(["check", "--base", base, "--strict", "--format", "sarif"])
        .args(arguments)
        .output()
        .map_err(|error| format!("{}: {error}", binary.display()))?;
    let code = output.status.code().unwrap_or(-1);
    if code == 3 || code < 0 {
        return Err(format!(
            "weed could not run on {} at {base}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse(&text).map_err(|error| format!("weed wrote SARIF weed's own reader refused: {error}"))
}

#[derive(Deserialize)]
struct Log {
    runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Run {
    results: Vec<Result_>,
}

#[derive(Deserialize)]
struct Result_ {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: Option<String>,
    #[serde(default)]
    locations: Vec<Location>,
}

#[derive(Deserialize)]
struct Location {
    #[serde(rename = "physicalLocation")]
    physical: Physical,
}

#[derive(Deserialize)]
struct Physical {
    #[serde(rename = "artifactLocation")]
    artifact: Artifact,
    region: Option<Region>,
}

#[derive(Deserialize)]
struct Artifact {
    uri: String,
}

#[derive(Deserialize)]
struct Region {
    #[serde(rename = "startLine")]
    start_line: Option<u32>,
}

fn parse(text: &str) -> Result<Vec<Finding>, String> {
    let log: Log = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let mut findings = Vec::new();
    for run in log.runs {
        for result in run.results {
            let (path, line) = match result.locations.first() {
                Some(location) => (
                    location.physical.artifact.uri.clone(),
                    location.physical.region.as_ref().and_then(|r| r.start_line),
                ),
                None => (String::new(), None),
            };
            findings.push(Finding {
                rule: result.rule_id,
                level: result.level.unwrap_or_else(|| "warning".to_string()),
                path,
                line,
            });
        }
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_is_read_with_its_place() {
        let text = r#"{"runs":[{"results":[{"ruleId":"T3","level":"error","locations":[
            {"physicalLocation":{"artifactLocation":{"uri":"a.test.ts"},"region":{"startLine":4}}}]}]}]}"#;
        let findings = parse(text).expect("the SARIF to read");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "T3");
        assert_eq!(findings[0].path, "a.test.ts");
        assert_eq!(findings[0].line, Some(4));
    }
}
