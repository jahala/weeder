//! S1, a stub or a TODO reached production code.
//!
//! The fire fixture writes every stub form the rule names into a production
//! file: the work markers, the language's own word for a body nobody wrote, and
//! a body whose only statement does nothing. The lines weed must report are read
//! off the fixture here by looking for those forms, so the expectation is
//! written independently of how the detector finds them.
//!
//! The neighbour writes the same tokens where they are honest: a `TODO` a test
//! carries about itself, and a `TODO` a production file holds inside a string
//! because the string is what it returns.

mod common;

use common::{fixture, fixture_file};

/// A language, the production file the stubs land in, and the forms it spells
/// them with.
struct Language {
    name: &'static str,
    path: &'static str,
    forms: &'static [&'static str],
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        path: "src/client.ts",
        forms: &["TODO", "FIXME", "XXX", "throw new Error(", "return null;"],
    },
    Language {
        name: "py",
        path: "src/client.py",
        forms: &["TODO", "FIXME", "XXX", "NotImplementedError", "pass", "..."],
    },
    Language {
        name: "rs",
        path: "src/client.rs",
        forms: &["TODO", "FIXME", "XXX", "todo!()", "unimplemented!()"],
    },
    Language {
        name: "go",
        path: "client.go",
        forms: &[
            "TODO",
            "FIXME",
            "XXX",
            "panic(\"not implemented\")",
            "return nil",
        ],
    },
];

/// The files the neighbour writes the same tokens in, and what makes each one
/// honest.
const NEIGHBOURS: [(&str, &str, &str); 4] = [
    ("ts", "src/client.test.ts", "src/client.ts"),
    ("py", "tests/test_client.py", "src/client.py"),
    ("rs", "tests/client.rs", "src/client.rs"),
    ("go", "client_test.go", "client.go"),
];

#[test]
fn s1_fires_at_block_level_on_every_stub_form_in_production_code() {
    for language in LANGUAGES {
        let repo = fixture("S1", language.name, "fire");
        let run = repo.weed(&["check"]);
        let name = language.name;

        assert_eq!(run.code, 2, "{name}: a stub blocks\n{}", run.stderr);
        let findings = run.findings();
        let expected = stub_lines(&language, "fire/after");
        assert!(
            expected.len() >= language.forms.len(),
            "{name}: the fixture must carry every form the rule names"
        );
        assert_eq!(
            findings
                .iter()
                .map(|finding| finding.line.expect("a stub finding names its line"))
                .collect::<Vec<u64>>(),
            expected,
            "{name}: every stub line is reported, and only those"
        );
        for finding in &findings {
            assert_eq!(finding.rule, "S1", "{name}: the rule is S1");
            assert_eq!(finding.level, "error", "{name}: S1 blocks");
            assert_eq!(finding.path, language.path, "{name}: the file is named");
        }
    }
}

#[test]
fn s1_stays_silent_in_a_test_file_and_on_a_marker_inside_a_string() {
    for (name, test_file, production) in NEIGHBOURS {
        let repo = fixture("S1", name, "silent");

        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        for path in [test_file, production] {
            assert!(
                changed.lines().any(|line| line == path),
                "{name}: {path} must be in the diff, or the silence proves nothing"
            );
        }
        assert!(
            fixture_file("S1", name, "silent/after", test_file).contains("TODO"),
            "{name}: the test file has to carry a marker, or it is not a neighbour"
        );
        assert!(
            fixture_file("S1", name, "silent/after", production).contains("\"TODO"),
            "{name}: the production file has to hold a marker inside a string"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a marker a test carries and a marker a string holds are not stubs"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// Where the forms the rule names are written in a fixture's production file,
/// counted from one and each line named once however many forms it carries.
fn stub_lines(language: &Language, case: &str) -> Vec<u64> {
    let source = fixture_file("S1", language.name, case, language.path);
    let mut lines: Vec<u64> = source
        .lines()
        .enumerate()
        .filter(|(_, line)| language.forms.iter().any(|form| line.contains(form)))
        .map(|(index, _)| index as u64 + 1)
        .collect();
    lines.sort_unstable();
    lines.dedup();
    lines
}
