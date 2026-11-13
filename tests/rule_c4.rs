//! C4, a gate was loosened in its own settings.
//!
//! An agent that cannot make a check pass can make the check unable to fail,
//! and every check behind it goes dark while the run stays green. Every case
//! runs the built binary on a real repository built from a fixture and reads
//! the SARIF it wrote.
//!
//! Two shapes block: a CI step that already ran a check told to let it fail,
//! in a workflow and in a pipeline file, and a manifest's or a makefile's
//! check script turned into one that cannot fail. Two warn: a gate's own
//! setting moved the loose way, and a check taken out of a workflow that runs
//! on a push or a pull request when no workflow runs it any more. Each has its
//! neighbour, the same key or the same `|| true` somewhere it decides nothing.

mod common;

use common::{fixture, fixture_file, weeder_in, Finding, Repo};
use tempfile::TempDir;
use weeder::core::catalogue::{self, Face};
use weeder::core::finding::Level;

const LANGS: [&str; 4] = ["ts", "py", "rs", "go"];

const WORKFLOW: &str = ".github/workflows/ci.yml";
const PIPELINE: &str = ".gitlab-ci.yml";

/// One loosening a fire fixture writes: the file, the text on the line the
/// finding has to name, and a check command the message has to name.
type Loosened = (&'static str, &'static str, &'static str);

/// What each fire fixture loosens.
const STEPS: [(&str, &[Loosened]); 4] = [
    (
        "ts",
        &[
            (WORKFLOW, "continue-on-error: true", "`npm test`"),
            (WORKFLOW, "npm run lint || true", "`npm run lint`"),
            (PIPELINE, "allow_failure: true", "`npm test`"),
        ],
    ),
    (
        "py",
        &[
            (WORKFLOW, "if: false", "`pytest`"),
            (WORKFLOW, "ruff check . || :", "`ruff check`"),
            (PIPELINE, "pytest; exit 0", "`pytest`"),
        ],
    ),
    (
        "rs",
        &[
            (WORKFLOW, "continue-on-error: true", "`cargo test`"),
            (PIPELINE, "allow_failure: true", "`cargo test`"),
        ],
    ),
    (
        "go",
        &[
            (WORKFLOW, "go test ./... || true", "`go test`"),
            (WORKFLOW, "if: ${{ false }}", "`go vet`"),
            (PIPELINE, "go test ./... || true", "`go test`"),
        ],
    ),
];

/// The check scripts each fire-script fixture makes a no-op: the file, and the
/// text on the line that names the script.
const SCRIPTS: [(&str, &[(&str, &str)]); 4] = [
    (
        "ts",
        &[
            ("package.json", "\"pretest\""),
            ("package.json", "\"test\":"),
            ("package.json", "\"test:unit\""),
            ("package.json", "\"lint\""),
        ],
    ),
    (
        "py",
        &[
            ("pyproject.toml", "test = "),
            ("pyproject.toml", "lint = "),
            ("pyproject.toml", "check = "),
        ],
    ),
    ("rs", &[("Makefile", "test:"), ("Makefile", "lint:")]),
    ("go", &[("Makefile", "test:"), ("Makefile", "check: test")]),
];

/// The gate settings each fire-setting fixture loosens: the file, and the text
/// on the line of the setting.
const SETTINGS: [(&str, &[(&str, &str)]); 4] = [
    (
        "ts",
        &[
            ("jest.config.js", "lines:"),
            ("tsconfig.json", "\"strict\""),
            (".eslintrc.json", "no-console"),
        ],
    ),
    (
        "py",
        &[
            ("pyproject.toml", "fail_under"),
            ("pyproject.toml", "ignore = "),
            ("pyproject.toml", "strict = "),
        ],
    ),
    (
        "rs",
        &[
            ("Cargo.toml", "unsafe_code"),
            ("Cargo.toml", "unwrap_used"),
            ("tarpaulin.toml", "fail-under"),
        ],
    ),
    (
        "go",
        &[
            (".golangci.yml", "disable:"),
            (".testcoverage.yml", "total:"),
        ],
    ),
];

/// The check each fire-dropped fixture takes out of its workflow.
const DROPPED: [(&str, &str); 4] = [
    ("ts", "`npm test`"),
    ("py", "`pytest`"),
    ("rs", "`cargo test`"),
    ("go", "`go test`"),
];

#[test]
fn c4_blocks_a_check_step_made_unable_to_fail_in_a_workflow_and_a_pipeline_in_every_language() {
    for (lang, expected) in STEPS {
        let repo = fixture("C4", lang, "fire");
        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.code, 2,
            "{lang}: a check that cannot fail stops the commit\n{}",
            run.stderr
        );
        let findings = of_c4(&run.findings());
        assert_eq!(
            located(&findings),
            expected_at(
                lang,
                "fire",
                expected.iter().map(|(path, text, _)| (*path, *text))
            ),
            "{lang}: every loosening is named on its own line, and nothing else is"
        );
        for (path, text, command) in expected {
            let line = line_of(lang, "fire", path, text);
            let finding = findings
                .iter()
                .find(|finding| finding.path == *path && finding.line == Some(line))
                .expect("the finding was matched above");
            assert_eq!(finding.level, "error", "{lang}: {path}:{line} blocks");
            assert!(
                finding.message.contains(command),
                "{lang}: the message names the check it silenced, {command}: {}",
                finding.message
            );
            assert!(
                finding.message.contains("Weeder-allow: C4"),
                "{lang}: the way through is the trailer: {}",
                finding.message
            );
        }
    }
}

#[test]
fn c4_stays_silent_on_a_step_that_checks_nothing_a_step_the_change_adds_and_a_true_after_nothing() {
    for lang in LANGS {
        let repo = fixture("C4", lang, "silent");
        let diff = repo.git(&["diff", "--cached", "--no-color"]);
        for shape in [
            "+        continue-on-error: true",
            "+        if: false",
            "+  allow_failure: true",
            "|| true",
        ] {
            assert!(
                diff.contains(shape),
                "{lang}: the neighbour has to add `{shape}`, or its silence proves nothing"
            );
        }
        let run = repo.weeder(&["check"]);
        assert_eq!(
            of_c4(&run.findings()),
            Vec::new(),
            "{lang}: an upload, a cache, a step the change adds and a `|| true` after a cleanup decide no check"
        );
    }
}

#[test]
fn a_weeder_allow_c4_trailer_stands_a_silenced_check_down_to_a_note() {
    for lang in LANGS {
        let repo = fixture("C4", lang, "fire");
        repo.pending_message(
            "Let the suite fail while upstream is broken\n\nWeeder-allow: C4 the runner image ships a broken toolchain until its next release\n",
        );
        let run = repo.weeder(&["check"]);
        let findings = of_c4(&run.findings());
        assert!(
            !findings.is_empty()
                && findings
                    .iter()
                    .all(|finding| finding.suppressed && finding.level == "note"),
            "{lang}: the trailer stands every finding down: {findings:?}"
        );
        assert_eq!(
            run.code, 0,
            "{lang}: nothing is left to block\n{}",
            run.stderr
        );
    }
}

#[test]
fn c4_blocks_a_check_script_made_a_no_op_in_every_language() {
    for (lang, expected) in SCRIPTS {
        let repo = fixture("C4", lang, "fire-script");
        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.code, 2,
            "{lang}: a no-op check script blocks\n{}",
            run.stderr
        );
        let findings = of_c4(&run.findings());
        assert_eq!(
            located(&findings),
            expected_at(lang, "fire-script", expected.iter().copied()),
            "{lang}: every script made a no-op is named where it is named, and no other"
        );
        for finding in &findings {
            assert_eq!(finding.level, "error", "{lang}: {} blocks", finding.message);
        }
    }
}

#[test]
fn c4_stays_silent_on_a_runner_swapped_a_flag_that_narrows_nothing_and_a_script_that_checks_nothing(
) {
    for lang in LANGS {
        let repo = fixture("C4", lang, "silent-script");
        let run = repo.weeder(&["check"]);
        assert_eq!(
            of_c4(&run.findings()),
            Vec::new(),
            "{lang}: a new runner, a flag beside the old command and a build script left alone are honest"
        );
    }
}

#[test]
fn c4_warns_on_a_gate_setting_loosened_in_every_language() {
    for (lang, expected) in SETTINGS {
        let repo = fixture("C4", lang, "fire-setting");
        let run = repo.weeder(&["check"]);
        let findings = of_c4(&run.findings());
        assert_eq!(
            located(&findings),
            expected_at(lang, "fire-setting", expected.iter().copied()),
            "{lang}: every setting moved the loose way is named on its line, and no other"
        );
        for finding in &findings {
            assert_eq!(
                finding.level, "warning",
                "{lang}: a setting has a reading a person can defend, so it warns: {}",
                finding.message
            );
            assert!(
                finding.message.contains("Weeder-allow: C4"),
                "{lang}: the reason goes in the trailer: {}",
                finding.message
            );
        }
    }
}

#[test]
fn c4_stays_silent_on_a_threshold_raised_a_switch_on_a_code_taken_out_and_a_dependency_called_ignore(
) {
    for lang in LANGS {
        let repo = fixture("C4", lang, "silent-setting");
        let run = repo.weeder(&["check"]);
        assert_eq!(
            of_c4(&run.findings()),
            Vec::new(),
            "{lang}: a gate tightened, and a key of the same name outside the gate, loosen nothing"
        );
    }
    let cargo = fixture_file("C4", "rs", "silent-setting/after", "Cargo.toml");
    assert!(
        cargo.contains("[dependencies]") && cargo.contains("ignore = \"0.4\""),
        "the rust neighbour adds a dependency called `ignore`"
    );
}

#[test]
fn c4_warns_when_a_check_leaves_a_gating_workflow_and_no_workflow_runs_it() {
    for (lang, command) in DROPPED {
        let repo = fixture("C4", lang, "fire-dropped");
        let run = repo.weeder(&["check"]);
        let findings = of_c4(&run.findings());
        assert_eq!(findings.len(), 1, "{lang}: one check left: {findings:#?}");
        let finding = &findings[0];
        assert_eq!(
            finding.path, WORKFLOW,
            "{lang}: the workflow it left is named"
        );
        assert_eq!(
            finding.level, "warning",
            "{lang}: a step may move, so it warns"
        );
        assert!(
            finding.message.contains(command),
            "{lang}: the message names {command}: {}",
            finding.message
        );
    }
}

#[test]
fn c4_stays_silent_on_a_check_that_still_runs_elsewhere_and_one_left_out_of_a_manual_workflow() {
    for lang in LANGS {
        let repo = fixture("C4", lang, "silent-dropped");
        let changed = repo.git(&["diff", "--cached", "--name-only"]);
        assert!(
            !changed.contains("tests.yml"),
            "{lang}: the workflow that still runs the suite is one the change leaves alone"
        );
        let run = repo.weeder(&["check"]);
        assert_eq!(
            of_c4(&run.findings()),
            Vec::new(),
            "{lang}: a check that runs in another workflow, or leaves one nobody gates on, is not gone"
        );
    }
}

#[test]
fn a_check_step_running_inside_a_shell_script_is_read_as_a_shell_reads_it() {
    let repo = Repo::init();
    let before = "on: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n    - run: bash -c \"cargo test\"\n    - run: |\n        cargo build \\\n          --release\n        cargo test --doc\n";
    repo.write(WORKFLOW, before);
    repo.commit("the state the change starts from");
    repo.write(
        WORKFLOW,
        &before
            .replace("bash -c \"cargo test\"", "bash -c \"cargo test || true\"")
            .replace("cargo test --doc", "cargo test --doc ||\n          true"),
    );
    let run = repo.weeder(&["check"]);
    let findings = of_c4(&run.findings());
    let lines: Vec<Option<u64>> = findings.iter().map(|finding| finding.line).collect();
    assert_eq!(
        lines,
        vec![Some(6), Some(10)],
        "a check inside `bash -c` and one carried onto the next line by `||` are both read: {findings:#?}"
    );
}

/// A workflow at the size one really is: this repository's own, with comments
/// between the steps, block values, expressions and three jobs. One check step
/// loosened is one finding naming it, and a `|| true` after a step that
/// checks nothing is none.
#[test]
fn a_real_workflow_is_read_step_by_step() {
    let own =
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(WORKFLOW))
            .expect("this repository's workflow reads");
    let step = "      - run: cargo test --workspace\n";
    let quiet = "      - run: sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0\n";
    assert!(
        own.contains(step) && own.contains(quiet),
        "the workflow still holds the two steps this case edits"
    );
    let repo = Repo::init();
    repo.write(WORKFLOW, &own);
    repo.commit("the state the change starts from");
    repo.write(
        WORKFLOW,
        &own.replacen(step, &format!("{step}        continue-on-error: true\n"), 1)
            .replace(quiet, &quiet.replace("=0\n", "=0 || true\n")),
    );
    let run = repo.weeder(&["check"]);
    let findings = of_c4(&run.findings());
    assert_eq!(findings.len(), 1, "one step loosened: {findings:#?}");
    assert!(
        findings[0]
            .message
            .contains("`cargo test` in the `check` job"),
        "the finding names the check and its job: {}",
        findings[0].message
    );
}

#[test]
fn the_catalogue_the_page_and_the_binary_all_carry_c4_as_a_check_rule_that_blocks() {
    let rule = catalogue::rule("C4").expect("the catalogue knows C4");
    assert_eq!(rule.face, Face::Check, "C4 judges a diff");
    assert_eq!(rule.default_level, Level::Block, "C4 blocks by default");

    let page = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/rules.md"),
    )
    .expect("the rules page is readable");
    assert!(
        page.contains("<a id=\"C4\"></a>"),
        "docs/rules.md carries an anchor for C4"
    );
    let anywhere = TempDir::new().expect("a directory to run in");
    let printed = weeder_in(anywhere.path(), &["rules"]);
    let row = printed
        .stdout_lines()
        .into_iter()
        .find(|line| line.starts_with("C4 "))
        .expect("`weeder rules` prints C4");
    let cells: Vec<&str> = row.split_whitespace().collect();
    assert_eq!(cells[1], "block", "the printed level");
    assert_eq!(cells[2], "check", "the printed face");
}

/// The findings C4 reported.
fn of_c4(findings: &[Finding]) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule == "C4")
        .cloned()
        .collect()
}

/// Where a set of findings points, sorted.
fn located(findings: &[Finding]) -> Vec<(String, u64)> {
    let mut at: Vec<(String, u64)> = findings
        .iter()
        .map(|finding| {
            (
                finding.path.clone(),
                finding
                    .line
                    .expect("a C4 finding of this shape names its line"),
            )
        })
        .collect();
    at.sort();
    at
}

/// Where the findings should point: each file and the line its text is on, read
/// from the fixture rather than from the detector.
fn expected_at<'a>(
    lang: &str,
    case: &str,
    texts: impl Iterator<Item = (&'a str, &'a str)>,
) -> Vec<(String, u64)> {
    let mut at: Vec<(String, u64)> = texts
        .map(|(path, text)| (path.to_string(), line_of(lang, case, path, text)))
        .collect();
    at.sort();
    at
}

/// The one line of a fixture's after side that carries the text.
fn line_of(lang: &str, case: &str, path: &str, text: &str) -> u64 {
    let after = fixture_file("C4", lang, &format!("{case}/after"), path);
    let lines: Vec<u64> = after
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(text))
        .map(|(index, _)| index as u64 + 1)
        .collect();
    assert_eq!(
        lines.len(),
        1,
        "{lang}/{case}: `{text}` names one line of {path}, found {lines:?}"
    );
    lines[0]
}
