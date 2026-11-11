//! T10, an added case holds a value the code defines to a literal.
//!
//! Each fire fixture adds one case whose only check compares a constant of the
//! code under test with the literal it is defined as. The four languages reach
//! that constant three ways: an import names it, in TypeScript and Python; the
//! file itself defines it outside its test module, in Rust; and Go's package
//! scope hands it to a test of the same package without any import at all.
//!
//! The neighbour adds a case in the same place that hands the same constant to
//! the code and checks what comes back against an independent literal. Both
//! fixtures name the constant and both write a literal; the difference is
//! whether anything the code computes stands between the two.

mod common;

use common::{fixture, fixture_file, Finding, Repo};

/// A language, the file its case is added to, the constant the case reads,
/// the line the fire fixture's check is written on, and the check the
/// neighbour writes instead.
struct Language {
    name: &'static str,
    test: &'static str,
    constant: &'static str,
    check_line: u64,
    echo: &'static str,
    computed: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        test: "src/service.test.ts",
        constant: "SERVICE",
        check_line: 10,
        echo: "expect(SERVICE).toBe(\"build.example.app\")",
        computed: "expect(endpoint(SERVICE)).toBe(\"https://build.example.app/\")",
    },
    Language {
        name: "py",
        test: "tests/test_service.py",
        constant: "SERVICE",
        check_line: 9,
        echo: "assert SERVICE == \"build.example.app\"",
        computed: "assert endpoint(SERVICE) == \"https://build.example.app/\"",
    },
    Language {
        name: "rs",
        test: "src/frame.rs",
        constant: "MAX_FRAME",
        check_line: 13,
        echo: "assert_eq!(MAX_FRAME, 64 * 1024)",
        computed: "assert_eq!(chunks(MAX_FRAME + 1), 2)",
    },
    Language {
        name: "go",
        test: "service_test.go",
        constant: "Service",
        check_line: 12,
        echo: "if Service != \"build.example.app\"",
        computed: "if got := Endpoint(Service); got != \"https://build.example.app/\"",
    },
];

#[test]
fn t10_warns_when_an_added_case_holds_a_constant_of_the_code_to_a_literal() {
    for language in LANGUAGES {
        let name = language.name;
        assert!(
            writes(&language, "fire", language.echo),
            "{name}: the fire fixture has to compare the constant with a literal"
        );
        let repo = fixture("T10", name, "fire");
        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one case that restates the code, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "warning", "{name}: T10 warns");
        assert_eq!(
            finding.path, language.test,
            "{name}: the finding lands on the file the case was added to"
        );
        assert_eq!(
            finding.line,
            Some(language.check_line),
            "{name}: and on the check that restates the code"
        );
        assert!(
            finding
                .message
                .contains(&format!("`{}`", language.constant)),
            "{name}: the finding names the constant: {}",
            finding.message
        );
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn t10_stays_silent_when_the_case_checks_what_the_code_makes_of_the_constant() {
    for language in LANGUAGES {
        let name = language.name;
        assert!(
            writes(&language, "silent", language.computed)
                && !writes(&language, "silent", language.echo),
            "{name}: the neighbour hands the constant to the code instead"
        );
        let repo = fixture("T10", name, "silent");
        assert!(
            changed(&repo).contains(&language.test.to_string()),
            "{name}: the neighbour adds its case to the same file"
        );
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a constant handed to the code and checked by what comes back is a test that can fail"
        );
    }
}

/// An existing case is never judged again: the fire fixture's case, already
/// committed, and a change elsewhere in the file leave T10 nothing to say.
#[test]
fn t10_judges_only_the_cases_a_change_adds() {
    for language in LANGUAGES {
        let name = language.name;
        let repo = fixture("T10", name, "fire");
        repo.stage_all();
        repo.commit("the echo lands");
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

/// Whether a fixture's test file writes that check after the change.
fn writes(language: &Language, case: &str, check: &str) -> bool {
    fixture_file(
        "T10",
        language.name,
        case,
        &format!("after/{}", language.test),
    )
    .contains(check)
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
        .filter(|finding| finding.rule == "T10")
        .cloned()
        .collect()
}
