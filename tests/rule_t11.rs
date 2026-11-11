//! T11, an added case asserts on nothing but a double it built itself.
//!
//! Each fire fixture adds one case that makes a double, tells it what to
//! answer, and then checks that it answered: a mock function in TypeScript, a
//! `Mock` in Python, a hand-written fake in Rust and in Go. Nothing the code
//! under test does stands between the configuration and the check.
//!
//! The neighbour builds the same double and hands it to the unit. Its checks
//! read the unit's output, and where one still reads the double, it reads what
//! the unit did to it, which is the effect the double was built to observe.

mod common;

use common::{fixture, fixture_file, Finding, Repo};

/// A language, the file its case is added to, the double the case builds, the
/// line the fire fixture's first check is written on, and the line the
/// neighbour hands the double to the unit with.
struct Language {
    name: &'static str,
    test: &'static str,
    double: &'static str,
    check_line: u64,
    handed: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        test: "src/editor.test.ts",
        double: "save",
        check_line: 12,
        handed: "const editor = new Editor(save);",
    },
    Language {
        name: "py",
        test: "tests/test_publish.py",
        double: "notify",
        check_line: 12,
        handed: "    publish(\"draft\", notify)",
    },
    Language {
        name: "rs",
        test: "tests/clock.rs",
        double: "clock",
        check_line: 33,
        handed: "let stamp = Stamp::from(&clock);",
    },
    Language {
        name: "go",
        test: "store_test.go",
        double: "fake",
        check_line: 21,
        handed: "report := Summarise(fake)",
    },
];

#[test]
fn t11_warns_when_an_added_case_asserts_only_on_the_double_it_built() {
    for language in LANGUAGES {
        let name = language.name;
        assert!(
            !writes(&language, "fire", language.handed),
            "{name}: the fire fixture never hands its double to the unit"
        );
        let repo = fixture("T11", name, "fire");
        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one case that checks its own double, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "warning", "{name}: T11 warns");
        assert_eq!(
            finding.path, language.test,
            "{name}: the finding lands on the file the case was added to"
        );
        assert_eq!(
            finding.line,
            Some(language.check_line),
            "{name}: and on the case's first check"
        );
        assert!(
            finding.message.contains(&format!("`{}`", language.double)),
            "{name}: the finding names the double: {}",
            finding.message
        );
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn t11_stays_silent_when_the_double_is_handed_to_the_unit_and_its_effect_checked() {
    for language in LANGUAGES {
        let name = language.name;
        assert!(
            writes(&language, "silent", language.handed),
            "{name}: the neighbour hands the same double to the unit"
        );
        let repo = fixture("T11", name, "silent");
        assert!(
            changed(&repo).contains(&language.test.to_string()),
            "{name}: the neighbour adds its case to the same file"
        );
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a double the unit is handed, and a check on what the unit did, can fail"
        );
    }
}

/// An existing case is never judged again: the fire fixture's case, already
/// committed, and a change elsewhere in the file leave T11 nothing to say.
#[test]
fn t11_judges_only_the_cases_a_change_adds() {
    for language in LANGUAGES {
        let name = language.name;
        let repo = fixture("T11", name, "fire");
        repo.stage_all();
        repo.commit("the self-check lands");
        let text =
            std::fs::read_to_string(repo.root().join(language.test)).expect("the test file reads");
        repo.write(language.test, &format!("{text}\n"));
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a case already committed is not the change's to answer for"
        );
    }
}

/// Whether a fixture's test file writes that line after the change.
fn writes(language: &Language, case: &str, line: &str) -> bool {
    fixture_file(
        "T11",
        language.name,
        case,
        &format!("after/{}", language.test),
    )
    .contains(line)
}

/// The paths the fixture's change touched, as git sees them.
fn changed(repo: &Repo) -> Vec<String> {
    repo.git(&["diff", "HEAD", "--name-only"])
        .lines()
        .map(str::to_string)
        .collect()
}

fn reported(findings: &[Finding]) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule == "T11")
        .cloned()
        .collect()
}
