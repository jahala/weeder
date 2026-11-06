//! T4, a tolerance or a timeout was widened.
//!
//! Each fire fixture rewrites two lines and nothing else: the number that says
//! how close a value has to be, and the number that says how long the test will
//! wait. The lines weeder must report are worked out here by comparing the two
//! sides of the fixture, so the expectation is "wherever the change is" rather
//! than a list copied out of the detector.
//!
//! The neighbour rewrites three lines the same way round: it tightens both
//! numbers instead of loosening them, and changes an expected value that is
//! nobody's tolerance.

mod common;

use common::{fixture, fixture_file, Finding};

/// A language and the test file its numbers live in.
struct Language {
    name: &'static str,
    path: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        path: "src/format.test.ts",
    },
    Language {
        name: "py",
        path: "tests/test_format.py",
    },
    Language {
        name: "rs",
        path: "tests/format.rs",
    },
    Language {
        name: "go",
        path: "format_test.go",
    },
];

#[test]
fn t4_warns_on_every_line_a_change_loosened() {
    for language in LANGUAGES {
        let repo = fixture("T4", language.name, "fire");
        let name = language.name;
        let rewritten = rewritten(&language, "fire");
        assert_eq!(
            rewritten.len(),
            2,
            "{name}: the fixture loosens a tolerance and a wait, and touches nothing else"
        );

        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings(), "T4");
        assert_eq!(
            findings
                .iter()
                .map(|finding| finding.line.expect("a T4 finding names its line"))
                .collect::<Vec<u64>>(),
            rewritten,
            "{name}: every loosened line is reported, and only those"
        );
        for finding in &findings {
            assert_eq!(finding.level, "warning", "{name}: T4 warns");
            assert_eq!(finding.path, language.path, "{name}: the file is named");
        }
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn t4_stays_silent_when_the_number_tightens_or_belongs_to_nobody() {
    for language in LANGUAGES {
        let repo = fixture("T4", language.name, "silent");
        let name = language.name;
        assert_eq!(
            rewritten(&language, "silent").len(),
            3,
            "{name}: the neighbour tightens two numbers and moves an expected value"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings(), "T4"),
            Vec::new(),
            "{name}: a number that tightens accepts less, and a literal that is nobody's slack says nothing"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// Each language moves two unchanged assertions one level deeper, into a
/// `describe`, a class, a module or a subtest. git writes that as the old lines
/// out and the new lines in, the block's opening line first, so a pairing by
/// order alone sets each assertion against the one written above it and reads
/// `0.3333` against `0.6667`. Nothing was loosened.
#[test]
fn t4_stays_silent_when_unchanged_assertions_are_only_reindented() {
    for language in LANGUAGES {
        let repo = fixture("T4", language.name, "reindented");
        let name = language.name;
        assert_eq!(
            loosened(&language, "reindented"),
            Vec::<u64>::new(),
            "{name}: every assertion the move touched reads the same once its indent is set aside"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings(), "T4"),
            Vec::new(),
            "{name}: a line that only moved deeper is the same line, and loosened nothing"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// The same move, with the second assertion loosened on the way in. The
/// assertion that only moved is no loosening, and the one that moved and
/// loosened is still reported, on its own line.
#[test]
fn t4_warns_on_a_loosening_beside_a_reindent() {
    for language in LANGUAGES {
        let repo = fixture("T4", language.name, "reindented-loosened");
        let name = language.name;
        let loosened = loosened(&language, "reindented-loosened");
        assert_eq!(
            loosened.len(),
            1,
            "{name}: the fixture loosens one assertion beside the one that only moved"
        );

        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings(), "T4");
        assert_eq!(
            findings
                .iter()
                .map(|finding| finding.line.expect("a T4 finding names its line"))
                .collect::<Vec<u64>>(),
            loosened,
            "{name}: the loosened line is reported, and the line that only moved is not"
        );
        for finding in &findings {
            assert!(
                !finding.message.contains("0.3333` became `0.6667"),
                "{name}: one assertion's value is never read against another's: {}",
                finding.message
            );
        }
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

/// The lines of the after side that rewrote a number and nothing else: the
/// line reads differently from every line of the before side once its indent
/// is set aside, and the same as one of them once its digits are set aside too.
/// A line that only moved deeper is in neither set, and the opening of the new
/// block is in the first alone.
fn loosened(language: &Language, case: &str) -> Vec<u64> {
    let read = |side: &str| {
        fixture_file(
            "T4",
            language.name,
            case,
            &format!("{side}/{}", language.path),
        )
    };
    let before = read("before");
    let after = read("after");
    let spaced = |line: &str| line.split_whitespace().collect::<Vec<&str>>().join(" ");
    let undigited = |line: &str| {
        spaced(line)
            .chars()
            .filter(|character| !character.is_ascii_digit())
            .collect::<String>()
    };
    after
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            before.lines().all(|was| spaced(was) != spaced(line))
                && before.lines().any(|was| undigited(was) == undigited(line))
        })
        .map(|(index, _)| index as u64 + 1)
        .collect()
}

/// The lines a fixture's change rewrote, counted from one. Both sides of these
/// fixtures hold the same lines in the same places, so a line that reads
/// differently is a line the change rewrote.
fn rewritten(language: &Language, case: &str) -> Vec<u64> {
    let before = fixture_file(
        "T4",
        language.name,
        case,
        &format!("before/{}", language.path),
    );
    let after = fixture_file(
        "T4",
        language.name,
        case,
        &format!("after/{}", language.path),
    );
    let before: Vec<&str> = before.lines().collect();
    let after: Vec<&str> = after.lines().collect();
    assert_eq!(
        before.len(),
        after.len(),
        "{}/{case}: the two sides have to line up, or the lines mean nothing",
        language.name
    );
    before
        .iter()
        .zip(&after)
        .enumerate()
        .filter(|(_, (was, now))| was != now)
        .map(|(index, _)| index as u64 + 1)
        .collect()
}

fn reported(findings: &[Finding], rule: &str) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule == rule)
        .cloned()
        .collect()
}
