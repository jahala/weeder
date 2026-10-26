//! T5, expected values were regenerated.
//!
//! The fire fixture is the pairing that reads badly: a production file changed
//! and the recorded expectation changed with it, in one commit. Each language
//! keeps its expectations where its own runner writes them, a snapshot
//! directory, a golden file under the data directory, and the fixture puts
//! them there rather than somewhere weeder would find convenient.
//!
//! Two neighbours, because there are two innocent halves. One moves the
//! expectation and leaves the code alone: a recording accepted on its own. The
//! other moves the code and leaves the expectation: a change the recording
//! still agrees with.
//!
//! Two more files live where expectations live and are none. A file the same
//! change adds was never an expectation anybody could rewrite: it was born with
//! the test that reads it, which is the test-first shape (#32). And a test
//! double is a program the tests run in place of a real one, which changes
//! with the code by design and records no output to drift (#65). weeder knows
//! one by what a program is: a file marked executable, or one whose first line
//! is a shebang.

mod common;

use common::{fixture, make_runnable, Finding, Repo};

/// A language, where its runner keeps the recorded expectation, and the
/// production file that expectation judges.
struct Language {
    name: &'static str,
    expectation: &'static str,
    production: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        expectation: "src/__snapshots__/format.test.ts.snap",
        production: "src/format.ts",
    },
    Language {
        name: "py",
        expectation: "tests/__snapshots__/test_format.ambr",
        production: "src/format.py",
    },
    Language {
        name: "rs",
        expectation: "tests/snapshots/format__pads_to_the_width.snap",
        production: "src/lib.rs",
    },
    Language {
        name: "go",
        expectation: "testdata/format.golden",
        production: "format.go",
    },
];

#[test]
fn t5_warns_on_an_expectation_that_moved_with_the_code_it_judges() {
    for language in LANGUAGES {
        let repo = fixture("T5", language.name, "fire");
        let name = language.name;
        let changed = changed(&repo);
        assert!(
            changed.contains(&language.expectation.to_string())
                && changed.contains(&language.production.to_string()),
            "{name}: the fixture has to move both halves: {changed:?}"
        );

        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings());
        assert_eq!(findings.len(), 1, "{name}: one expectation, one finding");
        let finding = &findings[0];
        assert_eq!(finding.level, "warning", "{name}: T5 warns");
        assert_eq!(
            finding.path, language.expectation,
            "{name}: the finding lands on the expectation"
        );
        assert!(
            finding.message.contains(language.production),
            "{name}: and names the code it moved with: {}",
            finding.message
        );
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn t5_stays_silent_when_only_the_expectation_moved() {
    for language in LANGUAGES {
        let repo = fixture("T5", language.name, "silent");
        let name = language.name;
        assert_eq!(
            changed(&repo),
            vec![language.expectation.to_string()],
            "{name}: the neighbour moves the expectation and nothing else"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a recording accepted on its own is nobody's agreement"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t5_stays_silent_when_only_the_code_moved() {
    for language in LANGUAGES {
        let repo = fixture("T5", language.name, "silent-code");
        let name = language.name;
        let changed = changed(&repo);
        assert!(
            changed.contains(&language.production.to_string())
                && !changed.contains(&language.expectation.to_string()),
            "{name}: the neighbour moves the code and leaves the expectation: {changed:?}"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: code the old expectation still agrees with is code nobody rewrote it for"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// Where each language's suite keeps the file the change adds for its new test.
const ADDED: [(&str, &str); 4] = [
    ("ts", "src/__fixtures__/padded.json"),
    ("py", "tests/fixtures/padded.json"),
    ("rs", "tests/fixtures/padded.txt"),
    ("go", "testdata/padded.txt"),
];

/// Where each language keeps a stand-in program its tests run.
const DOUBLES: [(&str, &str); 4] = [
    ("ts", "test/fixtures/fake-formatter.sh"),
    ("py", "tests/fixtures/fake_formatter.py"),
    ("rs", "tests/fixtures/fake-formatter.sh"),
    ("go", "testdata/fake-formatter.sh"),
];

/// The directories T5 reads as a store of recorded expectations. A neighbour
/// outside them would be silent for a reason this file is not about.
const STORES: [&str; 3] = ["__fixtures__", "fixtures", "testdata"];

#[test]
fn t5_stays_silent_on_an_expectation_the_same_change_adds() {
    for (language, (name, added)) in LANGUAGES.iter().zip(ADDED) {
        assert_eq!(
            language.name, name,
            "the tables walk the languages in one order"
        );
        let repo = fixture("T5", name, "silent-added");
        let status = statuses(&repo);
        assert!(
            status.contains(&format!("A\t{added}")),
            "{name}: the neighbour adds {added}: {status:?}"
        );
        assert!(
            status.contains(&format!("M\t{}", language.production)),
            "{name}: and changes the code beside it: {status:?}"
        );
        assert!(
            added.split('/').any(|segment| STORES.contains(&segment)),
            "{name}: {added} sits where T5 looks for expectations"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a file born with the test that reads it was never rewritten to agree"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t5_stays_silent_on_a_test_double_the_tests_run() {
    for (language, (name, double)) in LANGUAGES.iter().zip(DOUBLES) {
        assert_eq!(
            language.name, name,
            "the tables walk the languages in one order"
        );
        let repo = fixture("T5", name, "silent-double");
        let status = statuses(&repo);
        for path in [double, language.production] {
            assert!(
                status.contains(&format!("M\t{path}")),
                "{name}: the neighbour changes {path}: {status:?}"
            );
        }
        assert!(
            double.split('/').any(|segment| STORES.contains(&segment)),
            "{name}: {double} sits where T5 looks for expectations"
        );
        let first = std::fs::read_to_string(repo.root().join(double))
            .expect("the double should read")
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        assert!(
            first.starts_with("#!"),
            "{name}: the double opens with a shebang: {first}"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: a program the tests run records no output to drift"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// The stand-in the executable case writes, with no shebang to give it away.
const RUNNABLE: &str = "test/fixtures/fake-formatter";

#[test]
fn t5_reads_the_execute_bit_on_a_double_with_no_shebang() {
    for runnable in [true, false] {
        let repo = Repo::init();
        repo.write("src/format.ts", "export const pad = \" \";\n");
        repo.write(RUNNABLE, "echo \"a   \"\n");
        repo.stage_all();
        if runnable {
            make_runnable(&repo.root().join(RUNNABLE));
            repo.stage_all();
            // Windows keeps no execute bit on disk, so git is told outright.
            repo.git(&["update-index", "--chmod=+x", RUNNABLE]);
        }
        repo.git(&["commit", "-m", "the formatter and its stand-in"]);

        repo.write("src/format.ts", "export const pad = \".\";\n");
        repo.write(RUNNABLE, "echo \"a...\"\n");
        repo.stage_all();
        let mode = repo.git(&["ls-files", "--stage", RUNNABLE]);
        assert_eq!(
            mode.starts_with("100755"),
            runnable,
            "the index holds the mode this case is about: {mode}"
        );

        let found = reported(&repo.weeder(&["check"]).findings());
        if runnable {
            assert_eq!(
                found,
                Vec::new(),
                "a file marked executable is a program the tests run"
            );
        } else {
            assert_eq!(
                found
                    .iter()
                    .map(|finding| finding.path.as_str())
                    .collect::<Vec<_>>(),
                vec![RUNNABLE],
                "the same file without the bit is read as a recorded expectation"
            );
        }
    }
}

/// What the fixture's change did to each path, as git writes it: a status
/// letter, a tab, the path.
fn statuses(repo: &Repo) -> Vec<String> {
    repo.git(&["diff", "HEAD", "--name-status"])
        .lines()
        .map(str::to_string)
        .collect()
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
        .filter(|finding| finding.rule == "T5")
        .cloned()
        .collect()
}
