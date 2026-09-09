//! S1, a stub or a TODO reached production code.
//!
//! The fire fixture writes every stub form the rule names into a production
//! file: the work markers, the language's own word for a body nobody wrote, and
//! a body whose only statement does nothing. The lines weeder must report are read
//! off the fixture here by looking for those forms, so the expectation is
//! written independently of how the detector finds them.
//!
//! The neighbour writes the same tokens where they are honest: a `TODO` a test
//! carries about itself, a `TODO` a production file holds inside a string
//! because the string is what it returns, and the one body that does nothing on
//! purpose, a Python method under a protocol or an abstract base, where the
//! ellipsis is how the language states a type and there is nothing to finish.
//!
//! The last three cases are about a base and what fills it in. A method that
//! says it is not implemented is a stub where the work stops there and a
//! contract where something else in the tree writes it, so the neighbour puts
//! the override in a production file the change never touches, the fire fixture
//! puts one in a test file where it stands in for nothing a caller reaches, and
//! a third fixture states the base through an entry file, where the thing that
//! writes it may be in a repository weeder cannot read.

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
        let run = repo.weeder(&["check"]);
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

        let run = repo.weeder(&["check"]);
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

/// The Python neighbour's declaring classes: a protocol and an abstract base,
/// each with a method whose whole body is an ellipsis.
const DECLARED: [&str; 2] = ["Sink", "Store"];

/// The Python fire fixture's concrete pair: a function and a class method, both
/// with the same ellipsis body and nothing declaring it.
const CONCRETE: [&str; 2] = ["flush", "Buffer"];

#[test]
fn s1_stays_silent_on_an_ellipsis_that_declares_a_type_and_fires_on_one_that_defers() {
    let neighbour = fixture_file("S1", "py", "silent/after", "src/client.py");
    for class in DECLARED {
        let declared = ellipsis_bodies_under(&neighbour, class);
        assert!(
            !declared.is_empty(),
            "the neighbour has to write a method with an ellipsis body under `{class}`, or it is not a neighbour"
        );
    }

    let run = fixture("S1", "py", "silent").weeder(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "an ellipsis under a protocol or an abstract base is the declaration, not a body nobody wrote"
    );
    assert_eq!(run.code, 0, "nothing found, nothing blocked");

    let source = fixture_file("S1", "py", "fire/after", "src/client.py");
    let reported = fixture("S1", "py", "fire").weeder(&["check"]);
    let lines: Vec<u64> = reported
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == "S1")
        .filter_map(|finding| finding.line)
        .collect();
    for concrete in CONCRETE {
        let bodies = ellipsis_bodies_under(&source, concrete);
        assert!(
            !bodies.is_empty(),
            "the fire fixture has to write an ellipsis body under `{concrete}`"
        );
        for line in bodies {
            assert!(
                lines.contains(&line),
                "line {line}, an ellipsis under `{concrete}`, defers an implementation and is reported: {lines:?}"
            );
        }
    }
}

/// The 1-based lines holding an ellipsis body inside the declaration that names
/// `owner`: everything written further in than the line declaring it, up to the
/// next line written at the same indentation or less.
fn ellipsis_bodies_under(source: &str, owner: &str) -> Vec<u64> {
    let lines: Vec<&str> = source.lines().collect();
    let Some(start) = lines.iter().position(|line| {
        line.split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|word| word == owner)
    }) else {
        return Vec::new();
    };
    let opening = indent(lines[start]);
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate().skip(start + 1) {
        if line.trim().is_empty() {
            continue;
        }
        if indent(line) <= opening {
            break;
        }
        if line.trim() == "..." || line.trim_end().ends_with(": ...") {
            found.push(index as u64 + 1);
        }
    }
    found
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The base method each override case turns on: the file the base is declared
/// in, its name, the method the change leaves unfinished and how that method is
/// declared, the signal it leaves in the body, the production file elsewhere in
/// the tree that writes the method, the test file whose override counts for
/// nothing, and the entry file the exported case states the base through.
struct Contract {
    name: &'static str,
    base_file: &'static str,
    base: &'static str,
    declaration: &'static str,
    signal: &'static str,
    written_in: &'static str,
    test_override: &'static str,
    entry: &'static str,
}

const CONTRACTS: [Contract; 3] = [
    Contract {
        name: "ts",
        base_file: "src/client.ts",
        base: "Strategy",
        declaration: "retry(payload: string): string {",
        signal: "throw new Error(\"not implemented\");",
        written_in: "src/fast.ts",
        test_override: "src/client.test.ts",
        entry: "src/index.ts",
    },
    Contract {
        name: "py",
        base_file: "src/client.py",
        base: "Strategy",
        declaration: "def retry(self, payload):",
        signal: "raise NotImplementedError",
        written_in: "src/fast.py",
        test_override: "tests/test_client.py",
        entry: "src/__init__.py",
    },
    Contract {
        name: "rs",
        base_file: "src/client.rs",
        base: "Sink",
        declaration: "fn retry(&self, payload: &str) -> String {",
        signal: "todo!()",
        written_in: "src/buffer.rs",
        test_override: "tests/client.rs",
        entry: "src/lib.rs",
    },
];

#[test]
fn s1_stays_silent_where_something_in_the_tree_overrides_the_method() {
    for contract in CONTRACTS {
        let name = contract.name;
        let base = fixture_file("S1", name, "silent/after", contract.base_file);
        assert!(
            declares(&base, contract.base),
            "{name}: the neighbour has to declare `{}`, or it is not a neighbour",
            contract.base
        );
        assert!(
            base.contains(contract.declaration) && base.contains(contract.signal),
            "{name}: the base has to leave `{}` unfinished, or there is nothing to exempt",
            contract.declaration
        );
        let overriding = fixture_file("S1", name, "silent/after", contract.written_in);
        assert!(
            overriding.contains(contract.base) && overriding.contains(contract.declaration),
            "{name}: {} has to write the method the base left unfinished",
            contract.written_in
        );

        let repo = fixture("S1", name, "silent");
        let added = repo.git(&["diff", "HEAD", "--", contract.base_file]);
        assert!(
            added
                .lines()
                .any(|line| line.starts_with('+') && line.contains(contract.signal)),
            "{name}: the change has to add the signal, or the rule never looks at it"
        );
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            !changed.lines().any(|line| line == contract.written_in),
            "{name}: the override must sit outside the diff, or the tree is not what answered"
        );
        assert!(
            repo.git(&["ls-files"])
                .lines()
                .any(|line| line == contract.written_in),
            "{name}: the override has to be in the tree weeder reads"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a base method something in the tree writes is a contract, not a stub"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn s1_reports_a_base_method_nothing_overrides_and_counts_no_override_in_a_test_file() {
    for contract in CONTRACTS {
        let name = contract.name;
        let source = fixture_file("S1", name, "fire/after", contract.base_file);
        let doubled = fixture_file("S1", name, "fire/after", contract.test_override);
        assert!(
            doubled.contains(contract.base) && doubled.contains(contract.declaration),
            "{name}: the test file has to override the method, or the exclusion proves nothing"
        );
        let line = inside(&source, contract.base, contract.signal).unwrap_or_else(|| {
            panic!(
                "{name}: the fire fixture has to leave `{}` unfinished inside `{}`",
                contract.declaration, contract.base
            )
        });

        let run = fixture("S1", name, "fire").weeder(&["check"]);
        let finding = run
            .findings()
            .into_iter()
            .find(|finding| finding.line == Some(line))
            .unwrap_or_else(|| panic!("{name}: line {line} is a stub nothing writes"));
        assert_eq!(finding.rule, "S1", "{name}: the rule is S1");
        assert_eq!(
            finding.level, "error",
            "{name}: an override only a suite writes leaves the base unwritten"
        );
        assert!(
            finding
                .message
                .contains("nothing in the repository overrides it"),
            "{name}: the message says what weeder looked for: {}",
            finding.message
        );
    }
}

#[test]
fn s1_warns_where_the_base_is_exported_through_an_entry_file() {
    for contract in CONTRACTS {
        let name = contract.name;
        let entry = fixture_file("S1", name, "exported/after", contract.entry);
        assert!(
            entry.contains(contract.base),
            "{name}: {} has to state `{}`, or the base is not exported",
            contract.entry,
            contract.base
        );
        let source = fixture_file("S1", name, "exported/after", contract.base_file);
        let line = inside(&source, contract.base, contract.signal).unwrap_or_else(|| {
            panic!("{name}: the exported fixture has to leave the method unfinished")
        });

        let run = fixture("S1", name, "exported").weeder(&["check"]);
        assert_eq!(
            run.findings().len(),
            1,
            "{name}: one finding, on the method the change left unfinished"
        );
        let finding = &run.findings()[0];
        assert_eq!(finding.rule, "S1", "{name}: the rule is S1");
        assert_eq!(finding.line, Some(line), "{name}: on the signal's own line");
        assert_eq!(
            finding.level, "warning",
            "{name}: a base whose implementations may be elsewhere does not block"
        );
        assert!(
            finding.message.contains("another repository"),
            "{name}: the message says where the override may live: {}",
            finding.message
        );
        assert_eq!(run.code, 0, "{name}: a warning stops nothing");
    }
}

/// Whether a source declares a type of that name: the name written as a word on
/// a line that opens a block, which is how every language weeder reads spells a
/// declaration.
fn declares(source: &str, name: &str) -> bool {
    source.lines().any(|line| {
        line.split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|word| word == name)
            && (line.trim_end().ends_with('{') || line.trim_end().ends_with(':'))
    })
}

/// The 1-based line carrying `signal` inside the declaration that names `owner`:
/// the first one written further in than the line declaring it, before the next
/// line written at the same indentation or less.
fn inside(source: &str, owner: &str, signal: &str) -> Option<u64> {
    let lines: Vec<&str> = source.lines().collect();
    let start = lines.iter().position(|line| {
        line.split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|word| word == owner)
    })?;
    let opening = indent(lines[start]);
    for (index, line) in lines.iter().enumerate().skip(start + 1) {
        if line.trim().is_empty() {
            continue;
        }
        if indent(line) <= opening {
            return None;
        }
        if line.contains(signal) {
            return Some(index as u64 + 1);
        }
    }
    None
}
