//! T12, an added case passes a literal in and checks that it came back out.
//!
//! Each fire fixture adds one case that writes an error message into the
//! value it hands the code, and then checks that the code's answer holds the
//! same message. The code only has to pass the string along for the case to
//! pass, so the case says nothing about what the code makes of it.
//!
//! The neighbour hands the code the same message and checks the whole answer
//! against a literal of its own, the message inside the markup the code is
//! supposed to wrap it in. Both fixtures write the message twice; the
//! difference is whether the check holds the code to anything the code adds.

mod common;

use common::{fixture, fixture_file, Finding, Repo};

/// A language, the file its case is added to, the literal the case echoes,
/// the line the fire fixture's check compares on, and the check the neighbour
/// writes instead.
struct Language {
    name: &'static str,
    test: &'static str,
    literal: &'static str,
    check_line: u64,
    computed: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        test: "src/banner.test.ts",
        literal: "Workspace not found",
        check_line: 11,
        computed: "toBe('<p role=\"alert\">Workspace not found</p>')",
    },
    Language {
        name: "py",
        test: "tests/test_banner.py",
        literal: "Workspace not found",
        check_line: 10,
        computed: "assert banner(error) == \"<p>Workspace not found</p>\"",
    },
    Language {
        name: "rs",
        test: "tests/banner.rs",
        literal: "workspace not found",
        check_line: 11,
        computed: "assert_eq!(banner(&error), \"<p>workspace not found</p>\")",
    },
    Language {
        name: "go",
        test: "banner_test.go",
        literal: "workspace not found",
        check_line: 17,
        computed: "got != \"<p>workspace not found</p>\"",
    },
];

#[test]
fn t12_warns_when_an_added_case_checks_only_that_its_literal_came_back() {
    for language in LANGUAGES {
        let name = language.name;
        let written = fixture_file("T12", name, "fire", &format!("after/{}", language.test));
        assert_eq!(
            written.matches(language.literal).count(),
            2,
            "{name}: the fire fixture writes the literal once going in and once in the check"
        );
        let repo = fixture("T12", name, "fire");
        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one case that echoes its input, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "warning", "{name}: T12 warns");
        assert_eq!(
            finding.path, language.test,
            "{name}: the finding lands on the file the case was added to"
        );
        assert_eq!(
            finding.line,
            Some(language.check_line),
            "{name}: and on the check that reads the literal back"
        );
        assert!(
            finding.message.contains(language.literal),
            "{name}: the finding names the literal: {}",
            finding.message
        );
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn t12_stays_silent_when_the_case_checks_what_the_code_makes_of_its_input() {
    for language in LANGUAGES {
        let name = language.name;
        assert!(
            fixture_file("T12", name, "silent", &format!("after/{}", language.test))
                .contains(language.computed),
            "{name}: the neighbour checks the whole answer against its own literal"
        );
        let repo = fixture("T12", name, "silent");
        assert!(
            changed(&repo).contains(&language.test.to_string()),
            "{name}: the neighbour adds its case to the same file"
        );
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a check on what the code adds to its input can fail"
        );
    }
}

/// An existing case is never judged again: the fire fixture's case, already
/// committed, and a change elsewhere in the file leave T12 nothing to say.
#[test]
fn t12_judges_only_the_cases_a_change_adds() {
    for language in LANGUAGES {
        let name = language.name;
        let repo = fixture("T12", name, "fire");
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
        .filter(|finding| finding.rule == "T12")
        .cloned()
        .collect()
}
