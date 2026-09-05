//! The manifest the umbrella reads. F1 asks a bed to declare itself — name,
//! kind, version, install per platform, faces, the check command, the metric
//! command, the context cost — and the umbrella verifies the bed against what
//! it declared. A manifest is only worth reading if what it says is true, so
//! every claim here is checked against the thing it describes: the version
//! against `Cargo.toml`, the faces against what the binary prints, the check
//! command against the SARIF it actually emits.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{fixture, weed_command_in};
use jsonschema::Validator;
use serde_json::Value;

/// The metric F5 reads. The manifest names it; calibration's own check proves
/// it runs, which is why this file asks only that the name is right.
const METRIC_SCRIPT: &str = "scripts/check/calibration-bar.sh";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read(path: &str) -> String {
    let file = root().join(path);
    fs::read_to_string(&file).unwrap_or_else(|error| panic!("{path} should be readable: {error}"))
}

fn manifest() -> Value {
    serde_json::from_str(&read("garden.json")).expect("garden.json should be json")
}

fn validator(path: &str) -> Validator {
    let schema: Value = serde_json::from_str(&read(path))
        .unwrap_or_else(|error| panic!("{path} should be json: {error}"));
    jsonschema::validator_for(&schema)
        .unwrap_or_else(|error| panic!("{path} should compile as a schema: {error}"))
}

fn complaints(validator: &Validator, value: &Value) -> Vec<String> {
    validator
        .iter_errors(value)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect()
}

/// The version `Cargo.toml` carries, read from the file rather than from the
/// compiler, because the file is what the release workflow and the umbrella read.
fn cargo_version() -> String {
    let cargo: toml::Value = toml::from_str(&read("Cargo.toml")).expect("Cargo.toml should parse");
    cargo["package"]["version"]
        .as_str()
        .expect("Cargo.toml should name a package version")
        .to_string()
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("garden.json should carry a string at {key}"))
        .to_string()
}

/// The manifest names commands the way an operator types them: the binary by
/// its name, on the PATH. A test has a binary at a path instead, so the first
/// word is swapped for it and the rest is passed through untouched.
fn command_as_declared(declared: &str, directory: &Path) -> Command {
    let mut words = declared.split_whitespace();
    let program = words.next().expect("a command should have a first word");
    assert_eq!(
        program, "weed",
        "the check command should be weed's own, not {program}"
    );
    let arguments: Vec<&str> = words.collect();
    weed_command_in(directory, &arguments)
}

/// Every subcommand `weed --help` prints, which is what the binary offers a
/// caller. clap's own `help` is furniture rather than a face of weed.
fn subcommands_of_the_binary() -> Vec<String> {
    let printed = weed_command_in(&root(), &["--help"])
        .output()
        .expect("the weed binary should run");
    let text = String::from_utf8_lossy(&printed.stdout).to_string();
    let mut names: Vec<String> = section(&text, "Commands:")
        .iter()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_string)
        .collect();
    names.sort();
    assert!(
        !names.is_empty(),
        "weed --help printed no commands:\n{text}"
    );
    names
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

#[test]
fn the_manifest_validates_against_the_vendored_schema() {
    let complaints = complaints(&validator("schemas/garden.schema.json"), &manifest());
    assert!(
        complaints.is_empty(),
        "garden.json does not validate against schemas/garden.schema.json:\n{}",
        complaints.join("\n")
    );
}

#[test]
fn the_declared_version_is_the_version_cargo_carries() {
    assert_eq!(
        string(&manifest(), "version"),
        cargo_version(),
        "garden.json and Cargo.toml disagree about which weed this is"
    );
}

#[test]
fn the_declared_faces_are_the_subcommands_the_binary_offers() {
    let mut declared: Vec<String> = manifest()["faces"]
        .as_array()
        .expect("garden.json should carry a faces array")
        .iter()
        .map(|face| {
            face.as_str()
                .expect("a face is named by a string")
                .to_string()
        })
        .collect();
    declared.sort();
    assert_eq!(
        declared,
        subcommands_of_the_binary(),
        "garden.json declares faces the binary does not offer, or leaves out ones it does"
    );
}

#[test]
fn the_declared_check_command_emits_sarif_that_validates() {
    let manifest = manifest();
    let declared = string(&manifest["check"], "command");
    let repo = fixture("G1", "ts", "fire");
    let run = command_as_declared(&declared, repo.root())
        .output()
        .expect("the declared check command should run");

    let log: Value = serde_json::from_slice(&run.stdout).unwrap_or_else(|error| {
        panic!(
            "`{declared}` should write a SARIF log on stdout, but it did not parse: {error}\n{}",
            String::from_utf8_lossy(&run.stdout)
        )
    });
    let complaints = complaints(&validator("schemas/sarif-schema-2.1.0.json"), &log);
    assert!(
        complaints.is_empty(),
        "`{declared}` wrote a log that does not validate against SARIF 2.1.0:\n{}",
        complaints.join("\n")
    );
    // The declared format names a standard and its version — `sarif-2.1.0`.
    // The log carries that version in its own field, so the two are read
    // against each other rather than both being taken on trust.
    let declared_format = string(&manifest["check"], "format");
    let (standard, version) = declared_format
        .rsplit_once('-')
        .expect("the declared check format should name a standard and its version");
    assert_eq!(
        standard, "sarif",
        "weed's check face emits SARIF; garden.json declares {standard}"
    );
    assert_eq!(
        log["version"].as_str(),
        Some(version),
        "garden.json declares SARIF {version}, and the log says otherwise"
    );
}

#[test]
fn the_declared_metric_command_names_the_calibration_bar() {
    let declared = string(&manifest()["metric"], "command");
    assert!(
        declared
            .split_whitespace()
            .any(|word| word == METRIC_SCRIPT),
        "garden.json's metric command is `{declared}`, which does not name {METRIC_SCRIPT}"
    );
}

#[test]
fn the_declared_context_cost_is_zero() {
    assert_eq!(
        manifest()["context_cost"].as_u64(),
        Some(0),
        "weed is a binary a caller runs; it puts nothing in a session before it is called"
    );
}
