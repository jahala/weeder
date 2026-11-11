//! M1, a test mocks the unit it should exercise.
//!
//! Each fire fixture changes a production file and, in the same commit, a test
//! that stands a double in front of it. The languages name their doubles two
//! ways: the two that resolve a module hand the framework a specifier, a
//! relative path, a dotted module, and the two that resolve a type give the
//! double the name of what it replaces.
//!
//! The neighbour is the same change with the double pointed at a unit the
//! commit does not touch. The doubled file is in the repository both times, so
//! the difference between the two fixtures is what the change reaches, and
//! nothing else.
//!
//! The `fire-subject` fixtures ask the same question of a unit outside the
//! change: a case the change adds, with a double of what the test is about,
//! the file the test file is named for or the thing the case's title names.
//! Their neighbours, `silent-subject`, add the same case doubling a
//! collaborator the test is not about.

mod common;

use common::{fixture, fixture_file, Finding, Repo};

/// A language, the test file its double is set up in, the production file the
/// change edits, what a finding has to call the double, and the line each
/// fixture sets one up with.
struct Language {
    name: &'static str,
    test: &'static str,
    production: &'static str,
    named: &'static str,
    inside: &'static str,
    outside: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        test: "src/format.test.ts",
        production: "src/format.ts",
        named: "./format",
        inside: "vi.mock(\"./format\")",
        outside: "vi.mock(\"./clock\")",
    },
    Language {
        name: "py",
        test: "tests/test_format.py",
        production: "src/format.py",
        named: "src.format.format_value",
        inside: "@patch(\"src.format.format_value\")",
        outside: "@patch(\"src.clock.now\")",
    },
    Language {
        name: "rs",
        test: "tests/format.rs",
        production: "src/width.rs",
        named: "Width",
        inside: "MockWidth::new()",
        outside: "MockClock::new()",
    },
    Language {
        name: "go",
        test: "format_test.go",
        production: "format.go",
        named: "Formatter",
        inside: "NewMockFormatter(t)",
        outside: "NewMockClock(t)",
    },
];

#[test]
fn m1_warns_when_the_double_stands_in_for_a_file_the_change_edits() {
    for language in LANGUAGES {
        let repo = fixture("M1", language.name, "fire");
        let name = language.name;
        let changed = changed(&repo);
        assert!(
            changed.contains(&language.test.to_string())
                && changed.contains(&language.production.to_string()),
            "{name}: the change has to hold both the test and the unit: {changed:?}"
        );
        assert!(
            names_the_double(&language, "fire", language.inside),
            "{name}: the fire fixture has to double the unit the change edits"
        );

        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one double, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "warning", "{name}: M1 warns");
        assert_eq!(
            finding.path, language.test,
            "{name}: the finding lands on the test"
        );
        assert!(
            finding.message.contains(language.named)
                && finding.message.contains(language.production),
            "{name}: the finding names the double and the file it stands in for: {}",
            finding.message
        );
        assert!(
            finding.line.is_some_and(|line| line > 0),
            "{name}: and the line the double was set up on"
        );
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn m1_stays_silent_when_the_double_stands_in_for_something_outside_the_change() {
    for language in LANGUAGES {
        let repo = fixture("M1", language.name, "silent");
        let name = language.name;
        let changed = changed(&repo);
        assert!(
            changed.contains(&language.test.to_string())
                && changed.contains(&language.production.to_string()),
            "{name}: the neighbour changes the same two files: {changed:?}"
        );
        assert!(
            names_the_double(&language, "silent", language.outside)
                && !names_the_double(&language, "silent", language.inside),
            "{name}: and the double it sets up stands in for something else entirely"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a double of something the change never reaches is somebody's decision, not a finding"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// A language's subject fixtures: the test file a case is added to, the
/// production file the double stands in for, which the change leaves alone,
/// what a finding has to call the double, why the test is about it, and the
/// line each fixture sets its double up with.
struct Subject {
    name: &'static str,
    test: &'static str,
    production: &'static str,
    named: &'static str,
    because: &'static str,
    inside: &'static str,
    outside: &'static str,
}

const SUBJECTS: [Subject; 4] = [
    Subject {
        name: "ts",
        test: "src/format.test.ts",
        production: "src/format.ts",
        named: "./format",
        because: "the file the test is named for",
        inside: "vi.mock(\"./format\")",
        outside: "vi.mock(\"./clock\")",
    },
    Subject {
        name: "py",
        test: "tests/test_format.py",
        production: "src/format.py",
        named: "src.format.format_value",
        because: "test_format_value_cuts_a_long_value",
        inside: "patch(\"src.format.format_value\"",
        outside: "patch(\"src.format.now\"",
    },
    Subject {
        name: "rs",
        test: "tests/format.rs",
        production: "src/width.rs",
        named: "Width",
        because: "width_counts_the_characters",
        inside: "MockWidth::new()",
        outside: "MockClock::new()",
    },
    Subject {
        name: "go",
        test: "format_test.go",
        production: "format.go",
        named: "Formatter",
        because: "TestFormatterKeepsAShortValue",
        inside: "NewMockFormatter(t)",
        outside: "NewMockClock(t)",
    },
];

#[test]
fn m1_warns_when_an_added_case_doubles_what_the_test_is_about_outside_the_change() {
    for subject in SUBJECTS {
        let name = subject.name;
        let after = fixture_file(
            "M1",
            name,
            "fire-subject",
            &format!("after/{}", subject.test),
        );
        assert!(
            after.contains(subject.inside),
            "{name}: the fire fixture doubles what the test is about"
        );
        let repo = fixture("M1", name, "fire-subject");
        let changed = changed(&repo);
        assert!(
            changed == vec![subject.test.to_string()],
            "{name}: the change touches the test alone, and the unit stays as it was: {changed:?}"
        );

        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one double of the subject, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "warning", "{name}: M1 warns");
        assert_eq!(
            finding.path, subject.test,
            "{name}: the finding lands on the test"
        );
        assert!(
            finding.message.contains(subject.named) && finding.message.contains(subject.because),
            "{name}: the finding names the double and why the test is about it: {}",
            finding.message
        );
        assert!(
            !finding.message.contains(subject.production),
            "{name}: and claims no edit to a file the change left alone: {}",
            finding.message
        );
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn m1_stays_silent_when_the_added_case_doubles_a_collaborator_the_test_is_not_about() {
    for subject in SUBJECTS {
        let name = subject.name;
        let after = fixture_file(
            "M1",
            name,
            "silent-subject",
            &format!("after/{}", subject.test),
        );
        assert!(
            after.contains(subject.outside) && !after.contains(subject.inside),
            "{name}: the neighbour doubles something the test is not about"
        );
        let repo = fixture("M1", name, "silent-subject");
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a double of a collaborator is the test's own decision"
        );
    }
}

/// A double already committed is never judged again: the subject fixture's
/// double, landed, and a change elsewhere in the file leave M1 nothing to say.
#[test]
fn m1_judges_a_double_of_the_subject_only_where_the_change_sets_it_up() {
    for subject in SUBJECTS {
        let name = subject.name;
        let repo = fixture("M1", name, "fire-subject");
        repo.stage_all();
        repo.commit("the double lands");
        let text =
            std::fs::read_to_string(repo.root().join(subject.test)).expect("the test file reads");
        repo.write(subject.test, &format!("{text}\n"));
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a double already committed is not the change's to answer for"
        );
    }
}

/// A case named for the double it is about, `fn a_double_is_not_the_subject`,
/// writes the word in its own name, and its title is that name. The name is a
/// declaration, so it is never read as a double the case sets up. weeder's own
/// suite found this: its first run of the extension reported three of its own
/// test functions.
#[test]
fn m1_reads_a_case_named_for_a_double_as_a_name_and_not_a_double() {
    let cases: [(&str, &str); 3] = [
        (
            "py",
            "\n\ndef test_a_double_is_not_the_subject():\n    assert format_value(\"a\", 1) == \"a\"\n",
        ),
        (
            "rs",
            "\n#[test]\nfn a_double_is_not_the_subject() {\n    assert_eq!(Counted.of(\"a\"), 1);\n}\n",
        ),
        (
            "go",
            "\nfunc TestADoubleIsNotTheSubject(t *testing.T) {\n\tif got := (Padder{}).Format(\"a\", 1); got != \"a\" {\n\t\tt.Errorf(\"padding gave %q\", got)\n\t}\n}\n",
        ),
    ];
    for (name, case) in cases {
        let subject = SUBJECTS
            .iter()
            .find(|subject| subject.name == name)
            .expect("every language here has subject fixtures");
        let repo = fixture("M1", name, "silent-subject");
        repo.stage_all();
        repo.commit("the neighbour lands");
        let text =
            std::fs::read_to_string(repo.root().join(subject.test)).expect("the test file reads");
        repo.write(subject.test, &format!("{text}{case}"));
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a case's own name is not a double it sets up"
        );
    }
}

/// Whether a fixture's test file names that double after the change.
fn names_the_double(language: &Language, case: &str, double: &str) -> bool {
    fixture_file(
        "M1",
        language.name,
        case,
        &format!("after/{}", language.test),
    )
    .contains(double)
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
        .filter(|finding| finding.rule == "M1")
        .cloned()
        .collect()
}
