//! weeder's verdict on a pull request, as the one comment the action keeps.
//!
//! The comment is written by the binary, `weeder check --format markdown`, so
//! the order, the words and the links are weeder's own and the same change
//! writes the same bytes. It is read back here the way GitHub reads it, through
//! a CommonMark parser, and held to the SARIF log the same binary writes for the
//! same change: every result, blocks first, each with its rule, its level, its
//! message and a link to the line it names.
//!
//! The action around it is `action.yml`, and it is run here the way a runner
//! runs a composite action: its own steps, in order, handed to bash with the
//! environment GitHub gives them, in a real repository whose pull request
//! deletes a test. The weeder it installs is the one this suite built, packaged
//! by the release's own script and fetched by the action's own install, digest
//! and all. The one step not run here is the one that talks to GitHub; that half
//! is proved on a real pull request by `scripts/check/action-comment.sh`.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde_json::Value;
use weeder::core::sarif::{self, Context};
use weeder::core::{Finding, Level, Message, Region};

use common::{fixture, Repo, Run};

/// Where the links point: a server, a repository and the head commit, as the
/// action spells them for GitHub.
const LINKS: &str =
    "https://github.com/someone/somewhere/blob/0123456789abcdef0123456789abcdef01234567";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The comment and the log for one change, from two runs of the same binary.
fn both(repo: &Repo) -> (Run, Run) {
    let comment = repo.weeder(&["check", "--format", "markdown", "--link-base", LINKS]);
    let log = repo.weeder(&["check", "--format", "sarif"]);
    (comment, log)
}

/// A change with a block, a warning and a note in it: a test file deleted, a
/// workflow added, and a file in a language weeder carries no rules for.
fn mixed() -> Repo {
    let repo = fixture("T1", "rs", "fire");
    repo.write(
        ".github/workflows/build.yml",
        "name: build\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo built\n",
    );
    repo.write("tools/release.rb", "puts 'released'\n");
    repo.stage_all();
    repo
}

// --- the renderer ------------------------------------------------------------

#[test]
fn a_change_with_nothing_to_report_says_so_in_one_sentence() {
    let repo = fixture("T1", "rs", "silent");
    let (comment, log) = both(&repo);

    assert_eq!(comment.code, 0, "{}", comment.stderr);
    assert!(
        log.findings().is_empty(),
        "the fixture should be clean, and it found {:?}",
        log.findings()
    );
    let body = Body::read(&comment.stdout);
    assert!(
        body.items.is_empty(),
        "nothing found is no list:\n{}",
        comment.stdout
    );
    let said = body.prose.trim();
    assert!(
        !said.is_empty() && said.ends_with('.') && said.matches(". ").count() == 0,
        "nothing found is said in one sentence, and this is `{said}`"
    );
    assert!(
        said.contains("nothing to report"),
        "the sentence says plainly that there is nothing to report: `{said}`"
    );
}

#[test]
fn every_result_is_listed_blocks_first_with_its_rule_level_message_and_link() {
    let (comment, log) = both(&mixed());

    assert_eq!(
        comment.code, 2,
        "a deleted test blocks:\n{}",
        comment.stderr
    );
    assert_eq!(comment.code, log.code, "both formats reach one verdict");
    let results = ordered(&log.log());
    let levels: Vec<&str> = results
        .iter()
        .map(|result| result["level"].as_str().unwrap_or_default())
        .collect();
    for level in ["error", "warning", "note"] {
        assert!(
            levels.contains(&level),
            "the change should carry a result at {level}, and the levels were {levels:?}"
        );
    }

    let body = Body::read(&comment.stdout);
    assert_eq!(
        body.items.len(),
        results.len(),
        "one item per result:\n{}",
        comment.stdout
    );
    for (item, result) in body.items.iter().zip(&results) {
        let rule = result["ruleId"].as_str().unwrap_or_default();
        let level = result["level"].as_str().unwrap_or_default();
        let message = result["message"]["text"].as_str().unwrap_or_default();
        assert!(
            item.codes.iter().any(|code| code == rule),
            "the item names its rule `{rule}` as code: {item:?}"
        );
        assert!(
            item.strong.iter().any(|strong| strong == level),
            "the item names its level `{level}`: {item:?}"
        );
        assert!(
            unticked(&item.text).contains(&unticked(message)),
            "the item carries the message as the reader sees it.\nitem:    {}\nmessage: {message}",
            item.text
        );
        let (path, line, end) = place(result);
        let shown = match line {
            Some(line) => format!("{path}:{line}"),
            None => path.clone(),
        };
        // A note on the change as a whole names the repository, `.`, where a
        // blob address has no file to show, so it is shown and links nowhere.
        if path == "." {
            assert!(
                item.links.is_empty(),
                "the repository is no file to link: {item:?}"
            );
            assert!(
                item.codes.contains(&shown),
                "and is shown as written: {item:?}"
            );
            continue;
        }
        assert_eq!(
            item.links.first().map(|link| link.0.as_str()),
            Some(expected_link(&path, line, end).as_str()),
            "the item links to the line it names: {item:?}"
        );
        assert_eq!(
            item.links.first().map(|link| link.1.as_str()),
            Some(shown.as_str()),
            "the link reads as the place it names: {item:?}"
        );
    }
}

#[test]
fn a_path_markdown_would_read_is_linked_and_shown_as_written() {
    let repo = Repo::init();
    let odd = "tests/odd name [1]_#&.rs";
    repo.write(
        odd,
        "#[test]\nfn adds() {\n    assert_eq!(1 + 1, 2);\n}\n\n#[test]\nfn subtracts() {\n    assert_eq!(2 - 1, 1);\n}\n",
    );
    repo.commit("a suite with an awkward name");
    repo.remove(odd);
    repo.stage_all();

    let (comment, log) = both(&repo);
    let results = ordered(&log.log());
    assert!(
        results.iter().any(|result| place(result).0 == odd),
        "the deletion should be reported on `{odd}`: {results:?}"
    );
    let body = Body::read(&comment.stdout);
    assert_eq!(
        body.items.len(),
        results.len(),
        "one item per result:\n{}",
        comment.stdout
    );
    for (item, result) in body.items.iter().zip(&results) {
        let (path, line, end) = place(result);
        let link = item.links.first().expect("every item carries its link");
        assert_eq!(link.0, expected_link(&path, line, end));
        assert!(
            link.1.starts_with(&path),
            "the link shows `{path}` as written, and shows `{}`",
            link.1
        );
    }
}

#[test]
fn the_body_opens_with_a_hidden_marker_naming_weeder() {
    for repo in [fixture("T1", "rs", "silent"), mixed()] {
        let (comment, _) = both(&repo);
        let first = comment.stdout.lines().next().unwrap_or_default();
        assert!(
            first.starts_with("<!--") && first.ends_with("-->") && first.contains("weeder"),
            "the first line is the marker the action finds its comment by, and it is `{first}`"
        );
        let body = Body::read(&comment.stdout);
        assert!(
            body.hidden.iter().any(|html| html.trim() == first),
            "the marker is html a reader never sees: {:?}",
            body.hidden
        );
    }
}

#[test]
fn the_same_change_writes_the_same_bytes_in_every_locale_and_time_zone() {
    let repo = mixed();
    let arguments = ["check", "--format", "markdown", "--link-base", LINKS];
    let runs: Vec<Run> = [
        ("C", "UTC"),
        ("en_US.UTF-8", "Asia/Kathmandu"),
        ("tr_TR.UTF-8", "Pacific/Chatham"),
    ]
    .iter()
    .map(|(locale, zone)| {
        repo.weeder_with(
            &arguments,
            &[("LANG", locale), ("LC_ALL", locale), ("TZ", zone)],
        )
    })
    .collect();
    assert_eq!(runs[0].code, 2, "the change blocks: {}", runs[0].stderr);
    assert!(
        Body::read(&runs[0].stdout).items.len() >= 3,
        "the runs compared a comment with findings in it:\n{}",
        runs[0].stdout
    );
    for run in &runs[1..] {
        assert_eq!(
            run.stdout, runs[0].stdout,
            "the same change, the same bytes"
        );
        assert_eq!(run.code, runs[0].code);
    }
}

#[test]
fn a_run_that_could_not_judge_says_why_in_the_comment() {
    let repo = fixture("T1", "rs", "silent");
    let run = repo.weeder(&[
        "check",
        "--base",
        "no-such-ref",
        "--format",
        "markdown",
        "--link-base",
        LINKS,
    ]);

    assert_eq!(run.code, 3, "{}", run.stderr);
    let first = run.stdout.lines().next().unwrap_or_default();
    assert!(first.starts_with("<!--") && first.contains("weeder"));
    let body = Body::read(&run.stdout);
    let reason = run.stderr.trim();
    assert!(
        !reason.is_empty() && unticked(&body.prose).contains(&unticked(reason)),
        "the comment says why weeder could not judge.\nbody:   {}\nreason: {reason}",
        body.prose
    );
    assert!(
        body.prose.contains("could not judge"),
        "and says it judged nothing: {}",
        body.prose
    );
}

#[test]
fn links_are_refused_where_nothing_would_carry_them() {
    let repo = fixture("T1", "rs", "silent");
    let run = repo.weeder(&["check", "--format", "sarif", "--link-base", LINKS]);

    assert_eq!(run.code, 3, "a flag that would do nothing is refused");
    assert!(
        run.stderr.contains("--link-base") && run.stderr.contains("markdown"),
        "the refusal names the flag and the format it is for: {}",
        run.stderr
    );
}

/// What a finding's own words carry is written so a markdown reader shows it
/// as it was written: a placeholder in angle brackets is not html, a word with
/// underscores is not emphasis, a backtick nobody closed is not code, and a
/// name after an `@` notifies nobody.
#[test]
fn a_message_markdown_would_misread_reads_as_it_was_written() {
    let what = "a test named <RULE> and __init__ and *star* went, with `odd` and a lone ` and @someone on [a] line & more.";
    let finding = Finding {
        rule: "T1".to_string(),
        level: Level::Block,
        path: "tests/a.rs".to_string(),
        region: Some(Region {
            start_line: 3,
            end_line: 3,
        }),
        message: Message {
            what: what.to_string(),
            why: "the suite asks less.".to_string(),
            next: "restore it.".to_string(),
        },
        fix: None,
        suppressed: None,
    };
    let log = sarif::render(&[finding], &Context::new("0.0.0"));
    let body = Body::read(&weeder::core::comment::render(&log, Some(LINKS)));

    let item = body.items.first().expect("the finding is listed");
    let expected = format!("{what} the suite asks less. restore it.");
    assert!(
        unticked(&item.text).contains(&unticked(&expected)),
        "the message reads as written.\nitem:     {}\nexpected: {expected}",
        item.text
    );
    assert!(
        item.codes.iter().any(|code| code == "odd"),
        "a closed code span stays code: {item:?}"
    );
    assert!(
        item.plain.iter().all(|text| !text.contains("@someone")),
        "a name after an @ sits in code, where GitHub mentions nobody: {item:?}"
    );
    assert!(
        body.hidden.len() == 1,
        "the only html is the marker: {:?}",
        body.hidden
    );
}

// --- the action --------------------------------------------------------------

fn yaml(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} should read: {error}", path.display()));
    serde_yaml_ng::from_str(&text)
        .unwrap_or_else(|error| panic!("{} should be yaml: {error}", path.display()))
}

fn action() -> Value {
    yaml(&root().join("action.yml"))
}

fn example() -> Value {
    yaml(&root().join("examples/ci/github.yml"))
}

fn cargo_version() -> String {
    let manifest = std::fs::read_to_string(root().join("Cargo.toml")).expect("Cargo.toml reads");
    manifest
        .lines()
        .find_map(|line| line.strip_prefix("version = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("Cargo.toml names a version")
        .to_string()
}

#[test]
fn the_example_asks_for_what_the_comment_and_the_upload_need_and_nothing_else() {
    let workflow = example();
    let jobs = workflow["jobs"].as_object().expect("the example has jobs");
    assert_eq!(jobs.len(), 1, "the example is one job a reader copies");
    let job = jobs.values().next().expect("one job");

    let granted: BTreeMap<String, String> = job["permissions"]
        .as_object()
        .expect("the job states its permissions")
        .iter()
        .map(|(key, value)| (key.clone(), value.as_str().unwrap_or_default().to_string()))
        .collect();
    let wanted: BTreeMap<String, String> = [
        ("contents", "read"),
        ("pull-requests", "write"),
        ("security-events", "write"),
    ]
    .iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect();
    assert_eq!(
        granted, wanted,
        "the job asks for exactly what the comment and the upload need"
    );
    let above = workflow["permissions"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    for (key, value) in &above {
        assert!(
            key == "contents" && value == "read",
            "the workflow grants nothing above the job beyond reading: {key}: {value}"
        );
    }

    let uses: Vec<&str> = job["steps"]
        .as_array()
        .expect("the job has steps")
        .iter()
        .filter_map(|step| step["uses"].as_str())
        .collect();
    let tag = format!("jahala/weeder@v{}", cargo_version());
    assert!(
        uses.contains(&tag.as_str()),
        "the example runs the action by the tag of this version, `{tag}`, and uses {uses:?}"
    );
    let runs = job["steps"]
        .as_array()
        .expect("the job has steps")
        .iter()
        .filter(|step| step["run"].is_string())
        .count();
    assert_eq!(
        runs, 0,
        "the example installs and runs weeder through the action alone"
    );
}

#[test]
fn nothing_here_runs_on_pull_request_target() {
    let mut workflows = vec![root().join("examples/ci/github.yml")];
    for entry in std::fs::read_dir(root().join(".github/workflows")).expect("workflows read") {
        workflows.push(entry.expect("a workflow entry").path());
    }
    for path in workflows {
        let workflow = yaml(&path);
        let on = &workflow["on"];
        let named = match on {
            Value::String(event) => vec![event.clone()],
            Value::Array(events) => events
                .iter()
                .filter_map(|event| event.as_str().map(str::to_string))
                .collect(),
            Value::Object(events) => events.keys().cloned().collect(),
            _ => Vec::new(),
        };
        assert!(
            !named.iter().any(|event| event == "pull_request_target"),
            "{} runs on pull_request_target, which hands a fork's code a token that can write",
            path.display()
        );
    }
}

/// Code scanning takes the findings where it can (the owner's ruling of
/// 2026-11-13): from a public repository unless the pull request comes out of
/// a fork, whose token cannot upload, and from a private one only when it says
/// it has Advanced Security. The example passes the repository's own variable.
#[test]
fn the_upload_runs_where_code_scanning_can_take_it_and_nowhere_else() {
    let action = action();
    let input = &action["inputs"]["code-scanning"];
    assert_eq!(
        input["default"], "auto",
        "the input defaults to auto: {input}"
    );
    let upload = action["runs"]["steps"]
        .as_array()
        .expect("the action has steps")
        .iter()
        .find(|step| step["id"] == "upload")
        .expect("the action has an upload step");
    let uses = upload["uses"].as_str().unwrap_or_default();
    let (name, pin) = uses.split_once('@').expect("the upload names a version");
    assert_eq!(name, "github/codeql-action/upload-sarif");
    assert!(
        pin.len() == 40 && pin.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "the upload is pinned to a commit, not a tag: {uses}"
    );
    let condition: String = upload["if"]
        .as_str()
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for clause in [
        "inputs.code-scanning == 'on'",
        "inputs.code-scanning == 'auto'",
        "!github.event.repository.private",
        "github.event.pull_request.head.repo.full_name == github.repository",
    ] {
        assert!(
            condition.contains(clause),
            "the upload's condition holds `{clause}`: {condition}"
        );
    }
    assert_eq!(
        upload["with"]["sarif_file"], "${{ steps.judge.outputs.sarif }}",
        "it uploads the log the judge step wrote"
    );

    let example = example();
    let step = example["jobs"]
        .as_object()
        .and_then(|jobs| jobs.values().next())
        .and_then(|job| job["steps"].as_array())
        .and_then(|steps| {
            steps.iter().find(|step| {
                step["uses"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("jahala/weeder@")
            })
        })
        .expect("the example runs the action");
    assert_eq!(
        step["with"]["code-scanning"], "${{ vars.WEEDER_CODE_SCANNING || 'auto' }}",
        "the example lets a repository opt in through its own variable"
    );
}

#[test]
fn the_action_is_steps_a_runner_runs_in_order_with_no_condition_to_skip_one() {
    let action = action();
    assert_eq!(action["runs"]["using"], "composite");
    let steps = action["runs"]["steps"]
        .as_array()
        .expect("the action has steps");
    let ids: Vec<&str> = steps
        .iter()
        .map(|step| step["id"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        ids,
        ["install", "judge", "summary", "comment", "upload", "verdict"],
        "install, judge, then the summary, the comment and the upload, and the verdict last"
    );
    for step in steps {
        assert!(
            step.get("continue-on-error").is_none(),
            "no step is told it may fail: {step}"
        );
        if step["id"] == "upload" {
            continue;
        }
        assert!(
            step.get("if").is_none(),
            "no step but the upload decides by a condition whether it runs: {step}"
        );
        assert_eq!(step["shell"], "bash", "every step is bash: {step}");
        let script = step["run"].as_str().unwrap_or_default();
        assert!(
            !script.contains("${{"),
            "a step reads what GitHub hands it from its env, never spliced into the script: {step}"
        );
    }
}

/// The action's steps, run against a real repository the way a runner runs
/// them. The comment step is the one that talks to GitHub, and it is run here
/// with no token, the way a job with no right to write would run it.
#[cfg(unix)]
mod running {
    use super::*;

    use tempfile::TempDir;

    /// The world one run of the action sees: a runner's temp directory, the
    /// files a step appends its outputs, PATH and summary to, and a release
    /// directory holding the archive this suite built.
    struct Runner {
        scratch: TempDir,
        archive: String,
    }

    impl Runner {
        fn new() -> Runner {
            let scratch = TempDir::new().expect("a scratch directory");
            let release = scratch.path().join("release");
            let binary = common::binary();
            let directory = binary.parent().expect("the binary sits in a directory");
            let target = "this-machine";
            let packaged = common::bash(&root().join("scripts/package-release.sh"))
                .args([target, &directory.display().to_string()])
                .arg(&release)
                .output()
                .expect("the packaging script runs");
            assert!(
                packaged.status.success(),
                "the release's own packaging should package this build: {}",
                String::from_utf8_lossy(&packaged.stderr)
            );
            for name in ["temp", "home", "gh"] {
                std::fs::create_dir_all(scratch.path().join(name)).expect("a runner directory");
            }
            let archive = format!(
                "file://{}",
                release.join(format!("weeder-{target}.tar.gz")).display()
            );
            Runner { scratch, archive }
        }

        fn file(&self, name: &str) -> PathBuf {
            self.scratch.path().join(name)
        }

        fn read(&self, name: &str) -> String {
            std::fs::read_to_string(self.file(name)).unwrap_or_default()
        }
    }

    struct Step {
        id: String,
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// Every `${{ }}` an env block of the action may hold, filled in the way
    /// GitHub fills it for a pull request whose base is `main`.
    fn expressions(
        runner: &Runner,
        archive: &str,
        outputs: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let mut known: BTreeMap<String, String> = [
            ("inputs.token", ""),
            ("inputs.archive", archive),
            ("github.action_path", &root().display().to_string()),
            ("github.base_ref", "main"),
            ("github.event_name", "pull_request"),
            ("github.server_url", "https://github.com"),
            ("github.repository", "someone/somewhere"),
            ("github.event.pull_request.number", "7"),
            (
                "github.event.pull_request.head.sha",
                "0123456789abcdef0123456789abcdef01234567",
            ),
            ("runner.os", host_os()),
            ("runner.arch", host_arch()),
            ("runner.temp", &runner.file("temp").display().to_string()),
        ]
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
        for (key, value) in outputs {
            known.insert(key.clone(), value.clone());
        }
        known
    }

    fn host_os() -> &'static str {
        if cfg!(target_os = "macos") {
            "macOS"
        } else {
            "Linux"
        }
    }

    fn host_arch() -> &'static str {
        if cfg!(target_arch = "aarch64") {
            "ARM64"
        } else {
            "X64"
        }
    }

    fn resolve(text: &str, known: &BTreeMap<String, String>) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(start) = rest.find("${{") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 3..];
            let end = after.find("}}").expect("an expression closes");
            let expression = after[..end].trim();
            let value = known.get(expression).unwrap_or_else(|| {
                panic!("the action uses `{expression}`, which this run cannot fill in")
            });
            out.push_str(value);
            rest = &after[end + 2..];
        }
        out.push_str(rest);
        out
    }

    /// The action's steps run in the repository, each one only if the one
    /// before it passed, the way a composite action runs them.
    fn run_action(runner: &Runner, repo: &Repo, archive: &str) -> Vec<Step> {
        repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        let action = action();
        let mut outputs: BTreeMap<String, String> = BTreeMap::new();
        let mut ran = Vec::new();
        for (at, step) in action["runs"]["steps"]
            .as_array()
            .expect("the action has steps")
            .iter()
            .enumerate()
        {
            let id = step["id"].as_str().unwrap_or_default().to_string();
            // A step that uses another action runs only on GitHub, which this
            // suite never stands in for; the live pull request proves it.
            if step.get("uses").is_some() {
                continue;
            }
            let known = expressions(runner, archive, &outputs);
            let script = runner.file(&format!("step-{at}.sh"));
            // A runner hands a step to `bash --noprofile --norc -eo pipefail`;
            // the options are written into the script, so the harness's bash
            // runs it the same way.
            std::fs::write(
                &script,
                format!(
                    "set -eo pipefail\n{}",
                    step["run"].as_str().unwrap_or_default()
                ),
            )
            .expect("the step's script writes");
            let output_file = runner.file(&format!("output-{at}"));
            std::fs::write(&output_file, "").expect("an output file");
            let mut command = common::bash(&script);
            command
                .current_dir(repo.root())
                .env_clear()
                .env("PATH", path(runner))
                .env("HOME", runner.file("home"))
                .env("GH_CONFIG_DIR", runner.file("gh"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GITHUB_OUTPUT", &output_file)
                .env("GITHUB_PATH", runner.file("github-path"))
                .env("GITHUB_STEP_SUMMARY", runner.file("summary.md"))
                .env("RUNNER_TEMP", runner.file("temp"));
            if let Some(env) = step["env"].as_object() {
                for (key, value) in env {
                    command.env(key, resolve(value.as_str().unwrap_or_default(), &known));
                }
            }
            let output = command.output().expect("bash runs the step");
            for line in std::fs::read_to_string(&output_file)
                .unwrap_or_default()
                .lines()
            {
                if let Some((key, value)) = line.split_once('=') {
                    outputs.insert(format!("steps.{id}.outputs.{key}"), value.to_string());
                }
            }
            let code = output.status.code().unwrap_or(-1);
            ran.push(Step {
                id,
                code,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
            if code != 0 {
                break;
            }
        }
        ran
    }

    /// The system's own programs and nothing of this suite's: the weeder that
    /// judges is the one the action installed.
    fn path(runner: &Runner) -> String {
        let added: Vec<String> = runner
            .read("github-path")
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .rev()
            .collect();
        let mut entries = added;
        entries.push("/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin:/usr/local/bin".to_string());
        entries.join(":")
    }

    fn step<'a>(ran: &'a [Step], id: &str) -> &'a Step {
        ran.iter()
            .find(|step| step.id == id)
            .unwrap_or_else(|| panic!("the `{id}` step should have run; ran {:?}", ids(ran)))
    }

    fn ids(ran: &[Step]) -> Vec<&str> {
        ran.iter().map(|step| step.id.as_str()).collect()
    }

    #[test]
    fn the_action_installs_the_archive_it_names_after_checking_its_digest() {
        let runner = Runner::new();
        let repo = fixture("T1", "rs", "silent");
        let ran = run_action(&runner, &repo, &runner.archive);

        let install = step(&ran, "install");
        assert_eq!(install.code, 0, "{}{}", install.stdout, install.stderr);
        let path_file = runner.read("github-path");
        let installed: Vec<&str> = path_file.lines().map(str::trim).collect();
        assert_eq!(installed.len(), 1, "the install puts one directory on PATH");
        let beside: Vec<String> = std::fs::read_dir(installed[0])
            .expect("the installed directory reads")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert_eq!(
            beside,
            ["weeder"],
            "the install extracts the one executable"
        );
        assert_eq!(
            ids(&ran),
            ["install", "judge", "summary", "comment", "verdict"]
        );
        assert_eq!(step(&ran, "verdict").code, 0);
    }

    #[test]
    fn an_archive_whose_digest_does_not_match_is_never_opened() {
        let runner = Runner::new();
        let local = runner
            .archive
            .strip_prefix("file://")
            .expect("a file url")
            .to_string();
        let digest = format!("{local}.sha256");
        let written = std::fs::read_to_string(&digest).expect("the digest reads");
        let (hash, rest) = written.split_once(' ').expect("a digest and a name");
        let flipped: String = hash
            .chars()
            .map(|character| if character == '0' { '1' } else { '0' })
            .collect();
        std::fs::write(&digest, format!("{flipped} {rest}")).expect("the digest is replaced");

        let repo = fixture("T1", "rs", "silent");
        let ran = run_action(&runner, &repo, &runner.archive);
        let install = step(&ran, "install");
        assert_ne!(
            install.code, 0,
            "a digest that does not match stops the install"
        );
        assert!(
            install.stdout.contains("::error") && install.stdout.contains("SHA-256"),
            "and says why: {}{}",
            install.stdout,
            install.stderr
        );
        assert_eq!(
            ids(&ran),
            ["install"],
            "nothing runs after a refused install"
        );
        assert!(
            runner.read("github-path").trim().is_empty(),
            "nothing reaches PATH"
        );
    }

    #[test]
    fn a_blocked_change_fails_the_job_after_the_summary_and_the_comment_are_written() {
        let runner = Runner::new();
        let repo = fixture("T1", "rs", "fire");
        let ran = run_action(&runner, &repo, &runner.archive);

        assert_eq!(
            ids(&ran),
            ["install", "judge", "summary", "comment", "verdict"]
        );
        assert_eq!(
            step(&ran, "judge").code,
            0,
            "judging records the verdict and goes on"
        );
        let summary = runner.read("summary.md");
        assert!(
            summary
                .lines()
                .next()
                .is_some_and(|line| line.contains("weeder"))
                && summary.contains("`T1`"),
            "the summary carries the comment's body:\n{summary}"
        );
        let comment = step(&ran, "comment");
        assert_eq!(
            comment.code, 0,
            "a comment that cannot be written never changes the job's result: {}{}",
            comment.stdout, comment.stderr
        );
        assert!(
            comment.stdout.contains("::warning"),
            "the job says the comment was not written: {}",
            comment.stdout
        );
        assert_eq!(
            step(&ran, "verdict").code,
            2,
            "weeder's exit code decides the job"
        );
    }

    #[test]
    fn a_run_weeder_could_not_finish_fails_the_job_with_three() {
        let runner = Runner::new();
        let repo = fixture("T1", "rs", "silent");
        repo.write("weeder.toml", "[this is not toml\n");
        repo.commit("a law nobody can read");
        let ran = run_action(&runner, &repo, &runner.archive);

        assert_eq!(
            ids(&ran),
            ["install", "judge", "summary", "comment", "verdict"]
        );
        assert!(
            runner.read("summary.md").contains("could not judge"),
            "the summary says weeder judged nothing: {}",
            runner.read("summary.md")
        );
        assert_eq!(step(&ran, "verdict").code, 3);
    }

    #[test]
    fn the_install_reads_the_release_this_action_names_out_of_its_own_manifest() {
        let action = action();
        let install = action["runs"]["steps"]
            .as_array()
            .and_then(|steps| steps.iter().find(|step| step["id"] == "install"))
            .expect("an install step");
        let script = install["run"].as_str().unwrap_or_default();
        assert!(
            script.contains("garden.json") && script.contains("install.binaries"),
            "with no archive named, the release is the one garden.json names at the action's ref:\n{script}"
        );
        let manifest: Value = serde_json::from_str(
            &std::fs::read_to_string(root().join("garden.json")).expect("garden.json reads"),
        )
        .expect("garden.json is json");
        for target in [
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-musl",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
        ] {
            assert!(
                manifest["install"]["binaries"][target].is_string(),
                "garden.json names a release for {target}"
            );
            assert!(
                script.contains(target),
                "the install picks {target} for the runner it is on"
            );
        }
    }
}

// --- reading markdown back ---------------------------------------------------

/// A comment as a CommonMark reader sees it: the html it hides, the prose
/// outside any list, and each list item.
#[derive(Debug, Default)]
struct Body {
    hidden: Vec<String>,
    prose: String,
    items: Vec<Item>,
}

#[derive(Debug, Default, Clone)]
struct Item {
    /// Everything the item shows, code included, in order.
    text: String,
    /// The text the item shows outside code.
    plain: Vec<String>,
    codes: Vec<String>,
    strong: Vec<String>,
    /// Each link's destination and the text it shows.
    links: Vec<(String, String)>,
}

impl Body {
    fn read(markdown: &str) -> Body {
        let mut body = Body::default();
        let mut item: Option<Item> = None;
        let mut strong = false;
        let mut link: Option<(String, String)> = None;
        for event in Parser::new(markdown) {
            match event {
                Event::Html(html) | Event::InlineHtml(html) => body.hidden.push(html.to_string()),
                Event::Start(Tag::Item) => item = Some(Item::default()),
                Event::End(TagEnd::Item) => body.items.extend(item.take()),
                Event::Start(Tag::Strong) => strong = true,
                Event::End(TagEnd::Strong) => strong = false,
                Event::Start(Tag::Link { dest_url, .. }) => {
                    link = Some((dest_url.to_string(), String::new()))
                }
                Event::End(TagEnd::Link) => {
                    if let (Some(item), Some(link)) = (item.as_mut(), link.take()) {
                        item.links.push(link);
                    }
                }
                Event::Text(ref text) | Event::Code(ref text) => {
                    let code = matches!(event, Event::Code(_));
                    let text = text.to_string();
                    if let Some(link) = link.as_mut() {
                        link.1.push_str(&text);
                    }
                    match item.as_mut() {
                        Some(item) => {
                            item.text.push_str(&text);
                            if code {
                                item.codes.push(text);
                            } else {
                                if strong {
                                    item.strong.push(text.clone());
                                }
                                item.plain.push(text);
                            }
                        }
                        None => body.prose.push_str(&text),
                    }
                }
                Event::SoftBreak | Event::HardBreak => match item.as_mut() {
                    Some(item) => item.text.push(' '),
                    None => body.prose.push(' '),
                },
                Event::End(TagEnd::Paragraph) if item.is_none() => body.prose.push(' '),
                _ => {}
            }
        }
        body
    }
}

/// What a reader sees of a message, which is the message without the backticks
/// that mark its code, and with its spaces as a renderer folds them.
fn unticked(text: &str) -> String {
    text.replace('`', "")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// The results in the order the comment lists them: blocks, then warnings,
/// then notes, and inside a level in the order the log keeps.
fn ordered(log: &Value) -> Vec<Value> {
    let mut results = log["runs"][0]["results"]
        .as_array()
        .expect("a run carries results")
        .clone();
    let rank = |result: &Value| match result["level"].as_str() {
        Some("error") => 0,
        Some("warning") => 1,
        _ => 2,
    };
    results.sort_by_key(rank);
    results
}

fn place(result: &Value) -> (String, Option<u64>, Option<u64>) {
    let location = &result["locations"][0]["physicalLocation"];
    (
        location["artifactLocation"]["uri"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        location["region"]["startLine"].as_u64(),
        location["region"]["endLine"].as_u64(),
    )
}

/// `<links>/<path>#L<line>`, the path written byte by byte with everything but
/// the unreserved characters and `/` percent-encoded, and a range as
/// `#L<start>-L<end>`.
fn expected_link(path: &str, line: Option<u64>, end: Option<u64>) -> String {
    let mut encoded = String::new();
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    match (line, end) {
        (Some(line), Some(end)) => format!("{LINKS}/{encoded}#L{line}-L{end}"),
        (Some(line), None) => format!("{LINKS}/{encoded}#L{line}"),
        _ => format!("{LINKS}/{encoded}"),
    }
}
