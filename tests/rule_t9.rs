//! T9, a test case was added with no assertion.
//!
//! Each fire fixture adds one case to a file that already holds an honest one.
//! The new case calls the unit and checks nothing, so it passes whatever the
//! unit does. T2 counts assertions leaving a file and cannot see this, since
//! nothing left.
//!
//! The neighbour adds cases to the same file that assert in the shapes a
//! framework offers besides a plain assertion: a failure the case expects,
//! `toThrow`, `rejects`, `pytest.raises`, `#[should_panic]`, `t.Fatalf`; a Rust
//! case that returns a `Result` and hands a failure back with `?`; and a call
//! to a helper the same file defines, whose own body asserts. The reader's
//! outline names that helper and its lines, so following the call one level is
//! cheap. A helper that only calls another helper is not followed further.

mod common;

use common::{fixture, fixture_file, Finding, Repo};

/// A language, the file its case is added to, the line the fire fixture's case
/// opens on, the name the case is reported by, and what the neighbour writes
/// to assert without a plain assertion.
struct Language {
    name: &'static str,
    test: &'static str,
    case_line: u64,
    case: &'static str,
    honest: &'static [&'static str],
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        test: "src/parse.test.ts",
        case_line: 9,
        case: "trims a padded number",
        honest: &[
            ".toThrow(",
            ".rejects.toThrow(",
            "assert.rejects(",
            "trimsTo(\"",
        ],
    },
    Language {
        name: "py",
        test: "tests/test_parse.py",
        case_line: 8,
        case: "test_trims_a_padded_number",
        honest: &["pytest.raises(", "trims_to(\""],
    },
    Language {
        name: "rs",
        test: "src/parse.rs",
        case_line: 23,
        case: "trims_a_padded_number",
        honest: &[
            "#[should_panic(",
            "-> Result<(), String>",
            "?;",
            "trims_to(\"",
        ],
    },
    Language {
        name: "go",
        test: "parse_test.go",
        case_line: 11,
        case: "TestNormaliseTrimsAPaddedNumber",
        honest: &["t.Fatalf(", "t.Errorf(", "trimsTo(t,"],
    },
];

#[test]
fn t9_warns_when_an_added_case_asserts_nothing() {
    for language in LANGUAGES {
        let name = language.name;
        let repo = fixture("T9", name, "fire");
        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings());
        assert_eq!(
            findings.len(),
            1,
            "{name}: one case that checks nothing, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "warning", "{name}: T9 warns");
        assert_eq!(
            finding.path, language.test,
            "{name}: the finding lands on the file the case was added to"
        );
        assert_eq!(
            finding.line,
            Some(language.case_line),
            "{name}: and on the line the case opens on"
        );
        assert!(
            finding.message.contains(&format!("`{}`", language.case)),
            "{name}: the finding names the case: {}",
            finding.message
        );
        assert!(
            finding.message.contains("`Weeder-allow: T9"),
            "{name}: the next action names the trailer: {}",
            finding.message
        );
        assert_eq!(
            run.code, 0,
            "{name}: a warning is for the human at the pull request"
        );
    }
}

#[test]
fn t9_stays_silent_on_a_case_that_asserts_through_its_framework_or_a_helper_of_its_file() {
    for language in LANGUAGES {
        let name = language.name;
        let written = fixture_file("T9", name, "silent", &format!("after/{}", language.test));
        for shape in language.honest {
            assert!(
                written.contains(shape),
                "{name}: the neighbour asserts with `{shape}`"
            );
        }
        let repo = fixture("T9", name, "silent");
        assert!(
            changed(&repo).contains(&language.test.to_string()),
            "{name}: the neighbour adds its cases to the same file"
        );
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()),
            Vec::new(),
            "{name}: every added case asserts, through its framework or a helper"
        );
    }
}

/// The next action names the trailer, so the trailer has to do what it says:
/// stand the finding down to a note that still shows.
#[test]
fn t9_is_stood_down_by_a_trailer_that_says_why() {
    for language in LANGUAGES {
        let name = language.name;
        let repo = fixture("T9", name, "fire");
        repo.pending_message(
            "Add a smoke case for normalise\n\nWeeder-allow: T9 the case proves the call returns at all\n",
        );
        let run = repo.weeder(&["check"]);
        let findings = reported(&run.findings());
        assert!(
            !findings.is_empty()
                && findings
                    .iter()
                    .all(|finding| finding.suppressed && finding.level == "note"),
            "{name}: the trailer stands the finding down: {findings:?}"
        );
    }
}

/// An existing case is never judged again: the fire fixture's case, already
/// committed, and a change elsewhere in the file leave T9 nothing to say.
#[test]
fn t9_judges_only_the_cases_a_change_adds() {
    for language in LANGUAGES {
        let name = language.name;
        let repo = fixture("T9", name, "fire");
        repo.stage_all();
        repo.commit("the empty case lands");
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

/// A new file of green cases that check nothing is the other half of the
/// issue: every case in it is one the change added, and each is named.
#[test]
fn t9_names_every_case_of_a_new_file_that_asserts_nothing() {
    let repo = Repo::init();
    repo.write(
        "src/total.test.ts",
        "import { expect, it } from \"vitest\";\nimport { total } from \"./total\";\n\nit(\"adds two prices\", () => {\n  total([1, 2]);\n});\n\nit(\"adds no prices\", () => {\n  total([]);\n});\n\nit(\"adds one price\", () => {\n  expect(total([3])).toBe(3);\n});\n",
    );
    repo.stage_all();
    let run = repo.weeder(&["check"]);
    let findings = reported(&run.findings());
    let named: Vec<(Option<u64>, bool)> = findings
        .iter()
        .map(|finding| {
            (
                finding.line,
                finding.message.contains("`adds two prices`")
                    || finding.message.contains("`adds no prices`"),
            )
        })
        .collect();
    assert_eq!(
        named,
        [(Some(4), true), (Some(8), true)],
        "the two cases that check nothing, and not the one that does: {findings:?}"
    );
}

/// The helpers a repository asserts with from another file are what
/// `[tests] assertions` names, and a call to one is an assertion here as it is
/// for T2. Without the setting, the same case checks nothing weeder can see.
#[test]
fn t9_counts_a_call_to_a_helper_weeder_toml_names_in_every_language() {
    const HELPED: [(&str, &str, &str); 4] = [
        (
            "src/format.test.ts",
            "import { it } from \"vitest\";\nimport { check } from \"./checks\";\n\nit(\"pads to the width\", () => {\n  check(format(\"a\"), \"a \");\n});\n",
            "it(\"pads to the width\", () => {\n  format(\"a\");\n});\n",
        ),
        (
            "tests/test_format.py",
            "from tests.checks import check\n\n\ndef test_pads_to_the_width():\n    check(format(\"a\"), \"a \")\n",
            "def test_pads_to_the_width():\n    format(\"a\")\n",
        ),
        (
            "tests/format.rs",
            "mod checks;\nuse checks::check;\n\n#[test]\nfn pads_to_the_width() {\n    check(format(\"a\"), \"a \");\n}\n",
            "#[test]\nfn pads_to_the_width() {\n    format(\"a\");\n}\n",
        ),
        (
            "format_test.go",
            "package demo\n\nimport \"testing\"\n\nfunc TestPadsToTheWidth(t *testing.T) {\n\tcheck(t, Format(\"a\"), \"a \")\n}\n",
            "package demo\n\nimport \"testing\"\n\nfunc TestPadsToTheWidth(t *testing.T) {\n\tFormat(\"a\")\n}\n",
        ),
    ];
    for (path, helped, _) in HELPED {
        for named in [true, false] {
            let repo = Repo::init();
            if named {
                repo.write("weeder.toml", "[tests]\nassertions = [\"check\"]\n");
                repo.commit("the repository names its helper");
            }
            repo.write(path, helped);
            repo.stage_all();
            let run = repo.weeder(&["check"]);
            let findings = reported(&run.findings());
            assert_eq!(
                findings.len(),
                usize::from(!named),
                "{path}: named, `check` asserts; unnamed, it is a call like any other\n{}",
                run.output()
            );
        }
    }
    // And the case that calls the unit and no helper at all is reported
    // whatever the repository names.
    for (path, _, bare) in HELPED {
        let repo = Repo::init();
        repo.write("weeder.toml", "[tests]\nassertions = [\"check\"]\n");
        repo.commit("the repository names its helper");
        repo.write(path, bare);
        repo.stage_all();
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()).len(),
            1,
            "{path}: a case that calls no helper checks nothing\n{}",
            run.output()
        );
    }
}

/// A helper of the same file is followed one level: its own lines are read
/// for an assertion. A helper that only hands the value on to another helper
/// is not followed further, so that case is reported.
#[test]
fn t9_follows_a_helper_of_the_file_one_level_and_no_further() {
    let one = "import { expect, it } from \"vitest\";\nimport { total } from \"./total\";\n\nfunction totals(prices: number[], expected: number) {\n  expect(total(prices)).toBe(expected);\n}\n\nit(\"adds two prices\", () => {\n  totals([1, 2], 3);\n});\n";
    let two = "import { expect, it } from \"vitest\";\nimport { total } from \"./total\";\n\nfunction totals(prices: number[], expected: number) {\n  expect(total(prices)).toBe(expected);\n}\n\nfunction totalsPair(first: number, second: number) {\n  totals([first, second], first + second);\n}\n\nit(\"adds two prices\", () => {\n  totalsPair(1, 2);\n});\n";
    for (text, expected) in [(one, 0), (two, 1)] {
        let repo = Repo::init();
        repo.write("src/total.test.ts", text);
        repo.stage_all();
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()).len(),
            expected,
            "{text}\n{}",
            run.output()
        );
    }
}

/// A Rust case declared to return a `Result` asserts with each `?` that hands
/// a failure back. One whose body asks no question of anything has declared
/// the return and checks nothing with it.
#[test]
fn t9_reads_a_result_returning_rust_case_by_what_it_hands_back() {
    let head = "pub fn parse(input: &str) -> Result<u32, String> {\n    input.trim().parse().map_err(|_| String::new())\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn reads_a_number() -> Result<(), String> {\n";
    for (body, expected) in [
        ("        parse(\"7\")?;\n        Ok(())\n", 0),
        ("        parse(\"7\").ok();\n        Ok(())\n", 1),
    ] {
        let repo = Repo::init();
        repo.write("src/lib.rs", &format!("{head}{body}    }}\n}}\n"));
        repo.stage_all();
        let run = repo.weeder(&["check"]);
        assert_eq!(
            reported(&run.findings()).len(),
            expected,
            "{body}\n{}",
            run.output()
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
        .filter(|finding| finding.rule == "T9")
        .cloned()
        .collect()
}
