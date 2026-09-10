//! The manifest the umbrella reads. F1 asks a bed to declare itself, name,
//! kind, version, install per platform, faces, the check command, the metric
//! command, the context cost, the coverage its judgement has, and the umbrella
//! verifies the bed against what it declared. A manifest is only worth reading
//! if what it says is true, so every claim here is checked against the thing it
//! describes: the version against `Cargo.toml`, the cli against the binary, the
//! git hooks against what `weeder guard install` writes, the hook events
//! against what `weeder hook` decides, the check command against the SARIF it
//! actually emits, the coverage against what the classifier reads in this
//! repository's own files.
//!
//! `schemas/garden.schema.json` is no longer weeder's proposal: it is the
//! umbrella's `contracts/manifest.schema.json`, vendored whole at jahala/plotplot
//! v1.4.0, commit 106570d259b4116c32803c22b29ad67faf8dbaab, which is the
//! revision where the contract first allows `commit-msg` in `faces.git`; no tag
//! carries it, and v1.3.0 does not. `scripts/umbrella-brand.sh` fetches the
//! umbrella at the version `.brand/products/weeder/petalsrc.example` names,
//! which is how the copy is re-read against its source. The umbrella is a
//! private repository, so this file validates against the vendored copy and
//! holds it to the contract's own `$id` rather than reaching for the network in
//! the middle of `cargo test`.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{fixture, weeder_command_in, weeder_reading_in, Repo};
use jsonschema::Validator;
use serde_json::{json, Value};
use weeder::core::{classify_file, FileKind, Lang};

/// The metric F5 reads. The manifest names it; calibration's own check proves
/// it runs, which is why this file asks only that the name is right.
const METRIC_SCRIPT: &str = "scripts/check/calibration-bar.sh";
/// The contract the vendored schema is a copy of, as the contract names itself.
const CONTRACT_ID: &str = "https://plotplot.ai/contracts/manifest.schema.json";
/// The revision of the umbrella the vendored copy was taken at. weeder cannot
/// reach a private repository from a test, so the pin is held here and in
/// `garden.json`'s flagged note, and the two are read against each other.
const CONTRACT_REVISION: &str = "106570d259b4116c32803c22b29ad67faf8dbaab";
/// The git hook the ruling of 2026-09-10 added: the stage that reads an
/// allowance trailer, and so the stage that decides a block a trailer may allow.
const TRAILER_STAGE: &str = "commit-msg";
/// weeder's contract: 2 is at least one block-level result.
const REFUSED: i32 = 2;
/// The separator between a hook event and the tool call it is declared for,
/// `PreToolUse:Bash`. An event declared without one stands at every call.
const MATCHER: char = ':';

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

/// `Cargo.toml`, read from the file rather than from the compiler, because the
/// file is what the release workflow and the umbrella read.
fn cargo() -> toml::Value {
    toml::from_str(&read("Cargo.toml")).expect("Cargo.toml should parse")
}

fn cargo_package(key: &str) -> String {
    cargo()["package"][key]
        .as_str()
        .unwrap_or_else(|| panic!("Cargo.toml should name a package {key}"))
        .to_string()
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("garden.json should carry a string at {key}"))
        .to_string()
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("garden.json should carry an array at {key}"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("every entry of {key} is a string"))
                .to_string()
        })
        .collect()
}

/// The manifest names commands the way an operator types them: the binary by
/// its name, on the PATH. A test has a binary at a path instead, so the first
/// word is swapped for it and the rest is passed through untouched.
fn command_as_declared(declared: &str, directory: &Path) -> Command {
    let mut words = declared.split_whitespace();
    let program = words.next().expect("a command should have a first word");
    let cli = string(&manifest()["faces"], "cli");
    assert_eq!(
        program, cli,
        "the check command should be weeder's own cli, {cli}, not {program}"
    );
    let arguments: Vec<&str> = words.collect();
    weeder_command_in(directory, &arguments)
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

/// What the binary prints for a command, so a claim about weeder is read off
/// weeder rather than off a list somebody kept beside it.
fn help(arguments: &[&str]) -> String {
    let mut walked = arguments.to_vec();
    walked.push("--help");
    let printed = weeder_command_in(&root(), &walked)
        .output()
        .expect("the weeder binary should run");
    String::from_utf8_lossy(&printed.stdout).to_string()
}

/// The values clap prints for an argument that takes a fixed set of them.
fn possible_values(text: &str, heading: &str) -> BTreeSet<String> {
    section(text, heading)
        .iter()
        .filter_map(|line| line.split_once("[possible values: "))
        .filter_map(|(_, tail)| tail.split_once(']'))
        .flat_map(|(values, _)| {
            values
                .split(',')
                .map(|value| value.trim().to_string())
                .collect::<Vec<String>>()
        })
        .collect()
}

#[test]
fn the_manifest_validates_against_the_vendored_contract() {
    let schema: Value = serde_json::from_str(&read("schemas/garden.schema.json"))
        .expect("the vendored schema should be json");
    assert_eq!(
        schema["$id"].as_str(),
        Some(CONTRACT_ID),
        "schemas/garden.schema.json is the umbrella's manifest contract, vendored whole; a file \
         that names itself something else is a local proposal wearing the contract's filename"
    );

    let complaints = complaints(&validator("schemas/garden.schema.json"), &manifest());
    assert!(
        complaints.is_empty(),
        "garden.json does not validate against schemas/garden.schema.json:\n{}",
        complaints.join("\n")
    );
}

#[test]
fn the_vendored_contract_names_the_revision_it_was_taken_at() {
    let flagged = manifest()["flagged"]
        .as_array()
        .expect("garden.json should carry a flagged list")
        .iter()
        .find(|entry| string(entry, "what") == "schemas/garden.schema.json")
        .map(|entry| string(entry, "why"))
        .expect("the vendored schema is flagged until the umbrella tags the revision it came from");
    assert!(
        flagged.contains(CONTRACT_REVISION),
        "garden.json flags the vendored schema without saying which revision of the umbrella it \
         is: a copy nobody can re-read against its source is a fork wearing the contract's name\n\
         {flagged}"
    );
}

#[test]
fn the_contract_allows_the_stage_that_reads_an_allowance() {
    let schema: Value = serde_json::from_str(&read("schemas/garden.schema.json"))
        .expect("the vendored schema should be json");
    let allowed: Vec<String> = schema["properties"]["faces"]["properties"]["git"]["items"]["enum"]
        .as_array()
        .expect("the contract enumerates the git hooks a bed may declare")
        .iter()
        .map(|hook| hook.as_str().unwrap_or_default().to_string())
        .collect();
    assert!(
        allowed.iter().any(|hook| hook == TRAILER_STAGE),
        "the vendored contract does not allow {TRAILER_STAGE} in faces.git, so it is older than \
         {CONTRACT_REVISION}; re-fetch it at that revision rather than editing the copy"
    );
    assert!(
        strings(&manifest()["faces"], "git")
            .iter()
            .any(|hook| hook == TRAILER_STAGE),
        "garden.json does not declare {TRAILER_STAGE}, and the stem plants what the manifest \
         names: an allowance trailer would reach a hook nobody installed"
    );
}

#[test]
fn the_declared_name_and_version_are_the_crate_this_repository_builds() {
    let manifest = manifest();
    assert_eq!(
        string(&manifest, "name"),
        cargo_package("name"),
        "garden.json and Cargo.toml disagree about what this bed is called"
    );
    assert_eq!(
        string(&manifest, "version"),
        cargo_package("version"),
        "garden.json and Cargo.toml disagree about which weeder this is"
    );
}

#[test]
fn the_declared_install_names_what_this_repository_publishes() {
    let manifest = manifest();
    let install = &manifest["install"];
    let name = string(&manifest, "name");
    let version = string(&manifest, "version");

    assert_eq!(
        string(install, "cargo"),
        cargo_package("name"),
        "garden.json names a crate crates.io would not have under that name"
    );
    let npm: Value =
        serde_json::from_str(&read("npm/package.json")).expect("npm/package.json should be json");
    assert_eq!(
        string(install, "npm"),
        string(&npm, "name"),
        "garden.json names a package the npm wrapper does not publish"
    );
    // The wrapper is how most of the garden installs weeder, and it fetches the
    // release its own version names. A package.json left behind at the last
    // version publishes a wrapper that downloads the last binary.
    assert_eq!(
        string(&npm, "version"),
        version,
        "the npm wrapper publishes a version other than the one this repository builds"
    );

    let binaries = install["binaries"]
        .as_object()
        .expect("garden.json should carry install.binaries");
    for (target, url) in binaries {
        let url = url.as_str().expect("a target is downloaded from a url");
        let expected = format!("/releases/download/v{version}/{name}-{target}.tar.gz");
        assert!(
            url.ends_with(&expected),
            "garden.json points {target} at {url}, and the release builds {name}-{target}.tar.gz \
             for v{version}"
        );
    }
}

#[test]
fn the_declared_cli_is_the_binary_this_repository_builds() {
    let manifest = manifest();
    let cli = string(&manifest["faces"], "cli");
    let built = cargo()["bin"][0]["name"]
        .as_str()
        .expect("Cargo.toml should name a binary")
        .to_string();
    assert_eq!(
        cli, built,
        "garden.json names a command Cargo.toml does not build"
    );

    let printed = weeder_command_in(&root(), &["--version"])
        .output()
        .expect("the weeder binary should run");
    assert_eq!(
        String::from_utf8_lossy(&printed.stdout).trim(),
        format!("{cli} {}", string(&manifest, "version")),
        "the binary says it is something other than what the manifest declares"
    );
}

#[test]
fn the_declared_skill_is_the_file_a_bundle_would_copy() {
    let skill = string(&manifest()["faces"], "skill");
    let path = root().join(&skill);
    assert!(
        path.is_file(),
        "garden.json names {skill} as its skill, and the stem copies that path out of the release \
         artifact; nothing is there"
    );
    assert!(
        !read(&skill).trim().is_empty(),
        "{skill} is empty, so a harness that installs it learns nothing"
    );
}

#[test]
fn the_declared_git_hooks_are_the_hooks_guard_installs() {
    let repo = Repo::init();
    let run = repo.weeder(&["guard", "install"]);
    assert_eq!(run.code, 0, "guard install runs clean\n{}", run.stderr);

    let hooks_path = repo
        .git(&["config", "--local", "--get", "core.hooksPath"])
        .trim()
        .to_string();
    let written: BTreeSet<String> = fs::read_dir(repo.root().join(&hooks_path))
        .expect("guard install should have written a hooks directory")
        .map(|entry| {
            entry
                .expect("a hook on disk")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();

    let declared: BTreeSet<String> = strings(&manifest()["faces"], "git").into_iter().collect();
    assert_eq!(
        declared, written,
        "garden.json declares the git hooks weeder installs, and `weeder guard install` wrote \
         another set; the stem plants what the manifest names"
    );
}

#[test]
fn the_declared_harnesses_are_the_harnesses_the_hook_face_takes() {
    let declared: BTreeSet<String> = manifest()["faces"]["hooks"]
        .as_object()
        .expect("garden.json should carry faces.hooks")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        declared,
        possible_values(&help(&["hook"]), "Arguments:"),
        "garden.json declares hook entries for harnesses `weeder hook` does not answer, or leaves \
         out ones it does"
    );
}

#[test]
fn every_declared_hook_event_is_one_weeder_decides_at() {
    let manifest = manifest();
    let hooks = manifest["faces"]["hooks"]
        .as_object()
        .expect("garden.json should carry faces.hooks");

    for (harness, events) in hooks {
        for event in events
            .as_array()
            .expect("a harness declares a list of events")
        {
            let declared = event.as_str().expect("an event is named by a string");
            let (name, matcher) = match declared.split_once(MATCHER) {
                Some((name, matcher)) => (name, Some(matcher)),
                None => (declared, None),
            };
            // A tree weeder refuses, so an event it stands at has something to
            // refuse and an event it passes through has nothing to say.
            let repo = Repo::init();
            repo.write("src/parser.ts", &common::conflicted_parser(None));
            repo.stage_all();

            let run = weeder_reading_in(
                repo.root(),
                &["hook", harness],
                &hook_event(&repo, name, matcher),
            );
            assert_eq!(
                run.code,
                REFUSED,
                "garden.json declares {declared} for {harness}, and `weeder hook {harness}` let \
                 that event past over a tree that blocks: a declared event nobody stands at is a \
                 hook the stem wires up for nothing\n{}",
                run.output()
            );
        }
    }

    // The declaration is a list rather than a shrug: an event no harness
    // declares is passed through, over the same tree that refused above.
    for harness in hooks.keys() {
        let repo = Repo::init();
        repo.write("src/parser.ts", &common::conflicted_parser(None));
        repo.stage_all();
        let run = weeder_reading_in(
            repo.root(),
            &["hook", harness],
            &hook_event(&repo, "SessionStart", None),
        );
        assert_ne!(
            run.code,
            REFUSED,
            "`weeder hook {harness}` refuses at every event it is handed, so declaring two of \
             them says nothing\n{}",
            run.output()
        );
    }
}

/// An event in the shape a harness writes it: the name, the tool the entry is
/// declared for where it names one, and a command that asks git to make a
/// commit weeder would refuse. Every event carries the call and the end of the
/// turn together, because which of the two a name means is the harness's
/// business and not this test's.
fn hook_event(repo: &Repo, name: &str, matcher: Option<&str>) -> String {
    let mut event = json!({
        "session_id": "a-session",
        "transcript_path": null,
        "cwd": repo.root().display().to_string(),
        "hook_event_name": name,
        "stop_hook_active": false,
        "tool_input": {"command": "git commit -m 'the merge, half finished'"},
    });
    if let Some(tool) = matcher {
        event["tool_name"] = json!(tool);
    }
    event.to_string()
}

#[test]
fn the_declared_check_command_emits_sarif_that_validates() {
    let manifest = manifest();
    let declared = string(&manifest, "check");
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
    // F2 asks a gate for SARIF 2.1.0 by that name. The log carries the version
    // in its own field, and the schema it was just held to is the vendored copy
    // of that version, so the claim is read off the log rather than off the
    // manifest's word for it.
    assert_eq!(
        log["version"].as_str(),
        Some("2.1.0"),
        "the check face emits SARIF 2.1.0, and the log says otherwise"
    );
}

#[test]
fn the_declared_metric_command_names_the_calibration_bar() {
    let declared = string(&manifest(), "metric");
    assert!(
        declared
            .split_whitespace()
            .any(|word| word == METRIC_SCRIPT),
        "garden.json's metric command is `{declared}`, which does not name {METRIC_SCRIPT}"
    );
    assert!(
        root().join(METRIC_SCRIPT).is_file(),
        "garden.json names a metric command whose script is not in the tree"
    );
}

#[test]
fn the_declared_context_cost_is_a_measured_zero() {
    let manifest = manifest();
    assert_eq!(
        manifest["context"]["upfront_tokens"].as_u64(),
        Some(0),
        "weeder is a binary a caller runs; it puts nothing in a session before it is called"
    );
    assert!(
        manifest["faces"].get("mcp").is_none(),
        "a bed with an MCP face pays for its tool schemas at session start, and zero would be a \
         claim rather than a measurement"
    );
}

/// The manifest's word for a language weeder reads. The match is exhaustive on
/// purpose: a language added to the classifier and left unnamed here stops this
/// file compiling, which is what keeps a coverage claim from going quietly out
/// of date.
fn declared_language(lang: Lang) -> Option<&'static str> {
    match lang {
        Lang::TypeScript => Some("typescript"),
        Lang::JavaScript => Some("javascript"),
        Lang::Python => Some("python"),
        Lang::Rust => Some("rust"),
        Lang::Go => Some("go"),
        // Not a language weeder reads: the rules that fire on such a file read
        // its bytes and its path, never its syntax.
        Lang::Other => None,
    }
}

/// The manifest's word for a file kind weeder judges, exhaustive for the same
/// reason.
fn declared_kind(kind: FileKind) -> Option<&'static str> {
    match kind {
        FileKind::Test => Some("test"),
        FileKind::Prod => Some("prod"),
        FileKind::Config => Some("config"),
        FileKind::Manifest => Some("manifest"),
        FileKind::Generated => Some("generated"),
        FileKind::Guardrail => Some("guardrail"),
        FileKind::Workflow => Some("workflow"),
        // The remainder, which is the absence of a kind rather than one of them.
        FileKind::Other => None,
    }
}

/// Every file this repository tracks, as git lists them.
fn tracked() -> Vec<String> {
    let listed = Command::new("git")
        .arg("-C")
        .arg(root())
        .args(["ls-files", "-z"])
        .output()
        .expect("git should be on PATH");
    assert!(
        listed.status.success(),
        "git should list this repository's files:\n{}",
        String::from_utf8_lossy(&listed.stderr)
    );
    String::from_utf8_lossy(&listed.stdout)
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn the_declared_coverage_is_what_the_classifier_reads_in_this_repository() {
    let mut languages = BTreeSet::new();
    let mut kinds = BTreeSet::new();
    for path in tracked() {
        let Ok(bytes) = fs::read(root().join(&path)) else {
            // A path git tracks and the working tree does not hold is a
            // submodule or a broken link; there is nothing to classify.
            continue;
        };
        let classification = classify_file(&path, &String::from_utf8_lossy(&bytes));
        if let Some(name) = declared_language(classification.lang) {
            languages.insert(name.to_string());
        }
        if let Some(name) = declared_kind(classification.kind) {
            kinds.insert(name.to_string());
        }
    }

    let manifest = manifest();
    let coverage = &manifest["coverage"];
    assert_eq!(
        strings(coverage, "languages")
            .into_iter()
            .collect::<BTreeSet<String>>(),
        languages,
        "garden.json declares the languages weeder's judgement covers, and the classifier reads \
         another set out of this repository's own files. Silence on a language a bed does not \
         cover has to be declared, or it reads as clean"
    );
    assert_eq!(
        strings(coverage, "kinds")
            .into_iter()
            .collect::<BTreeSet<String>>(),
        kinds,
        "garden.json declares the file kinds weeder judges, and the classifier assigns another set \
         to this repository's own files"
    );
}
