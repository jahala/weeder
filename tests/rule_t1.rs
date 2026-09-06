//! T1, a test was deleted.
//!
//! Two shapes, in four languages. A test file removed takes every case it held;
//! a test file that stayed and declares fewer cases lost the difference. The
//! neighbour does the two honest things that look the same from a distance, a
//! case renamed where it stands, and a case added, and weed says nothing.
//!
//! A third pair reads the move a growing suite makes: the cases stop being
//! written out one at a time and come from a table instead, a `for` around an
//! `it`, a `t.Run` in a range, a `parametrize` list, a macro invoked once per
//! line. Nothing is covered less afterwards, so weed counts the entries and
//! says nothing; a table that leaves a case behind is still a case gone.
//!
//! A fourth pair reads the move a suite makes between files. Splitting one file
//! into two, or folding two into one, takes cases out of one file and puts them
//! into another in the same diff, some under their own name and some renamed on
//! the way, and nothing is covered less afterwards. A case that went nowhere is
//! a case gone like any other, and the finding that reports it counts only what
//! left and names the files the rest moved into.
//!
//! Every number here is counted off the fixture by this file's own reading of
//! what a case looks like, so the expectation is written independently of the
//! detector that has to meet it.

mod common;

use common::{fixture, fixture_file};

/// A language, the test file the change deletes, and the test file it thins.
struct Language {
    name: &'static str,
    deleted: &'static str,
    thinned: &'static str,
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        deleted: "src/parser.test.ts",
        thinned: "src/format.test.ts",
    },
    Language {
        name: "py",
        deleted: "tests/test_parser.py",
        thinned: "tests/test_format.py",
    },
    Language {
        name: "rs",
        deleted: "tests/parser.rs",
        thinned: "tests/format.rs",
    },
    Language {
        name: "go",
        deleted: "parser_test.go",
        thinned: "format_test.go",
    },
];

#[test]
fn t1_fires_at_block_level_on_a_deleted_test_file_and_a_thinned_one() {
    for language in LANGUAGES {
        let repo = fixture("T1", language.name, "fire");
        let run = repo.weed(&["check"]);
        let name = language.name;

        assert_eq!(run.code, 2, "{name}: a deleted test blocks\n{}", run.stderr);
        let (findings, others): (Vec<common::Finding>, Vec<common::Finding>) = run
            .findings()
            .into_iter()
            .partition(|finding| finding.rule == "T1");
        assert_eq!(
            findings.len(),
            2,
            "{name}: the file that went and the file that shrank, and nothing else: {findings:#?}"
        );
        for finding in &findings {
            assert_eq!(finding.level, "error", "{name}: T1 blocks");
        }
        // A case that goes takes its assertions with it, so T2 reads the file
        // that shrank and says its own thing about it. Nothing else may.
        assert!(
            others
                .iter()
                .all(|finding| finding.rule == "T2" && finding.path == language.thinned),
            "{name}: the only other reading of this change is the assertions that went with the case: {others:#?}"
        );
        assert_eq!(
            sorted_paths(&findings),
            sorted(vec![language.deleted, language.thinned]),
            "{name}: both files are named"
        );

        let gone = finding_on(&findings, language.deleted);
        let held = cases(name, "fire/before", language.deleted);
        assert!(
            gone.message.contains(&held.to_string()),
            "{name}: the finding says how many cases went with the file: {}",
            gone.message
        );
        assert!(
            gone.message.contains(COVERED_BY_NOBODY),
            "{name}: nothing in this diff took the cases over, so the finding may say so: {}",
            gone.message
        );

        let thinned = finding_on(&findings, language.thinned);
        let before = cases(name, "fire/before", language.thinned);
        let after = cases(name, "fire/after", language.thinned);
        assert!(
            after < before,
            "{name}: the fixture has to lose a case, or the check proves nothing"
        );
        assert!(
            thinned.message.contains(&before.to_string())
                && thinned.message.contains(&after.to_string()),
            "{name}: the finding says what the count was and what it is: {}",
            thinned.message
        );
    }
}

#[test]
fn t1_stays_silent_when_a_case_is_renamed_in_place_or_a_new_one_is_added() {
    for language in LANGUAGES {
        let repo = fixture("T1", language.name, "silent");
        let name = language.name;

        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path == language.thinned),
            "{name}: the neighbour must be in the diff, or the silence proves nothing"
        );
        let before = cases(name, "silent/before", language.thinned);
        let after = cases(name, "silent/after", language.thinned);
        assert!(
            after > before,
            "{name}: the neighbour renames one case and adds another, so the count rises"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a rename in place and a new case take nothing away"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

fn finding_on(findings: &[common::Finding], path: &str) -> common::Finding {
    findings
        .iter()
        .find(|finding| finding.path == path)
        .unwrap_or_else(|| panic!("a finding on {path}, among {findings:#?}"))
        .clone()
}

/// The files a set of findings named, sorted and without repeats.
fn sorted_paths(findings: &[common::Finding]) -> Vec<String> {
    let mut paths: Vec<String> = findings
        .iter()
        .map(|finding| finding.path.clone())
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

fn sorted(mut paths: Vec<&str>) -> Vec<String> {
    paths.sort_unstable();
    paths.into_iter().map(ToString::to_string).collect()
}

/// How many cases a fixture's test file declares, counted the way each language
/// writes one down.
fn cases(lang: &str, case: &str, path: &str) -> usize {
    let source = fixture_file("T1", lang, case, path);
    source
        .lines()
        .map(str::trim)
        .filter(|line| match lang {
            "ts" => line.starts_with("it("),
            "py" => line.starts_with("def test"),
            "rs" => *line == "#[test]",
            "go" => line.starts_with("func Test"),
            other => panic!("no case shape is written down for {other}"),
        })
        .count()
}

/// A language, its test file, and the entries the table in `after/` holds.
struct Table {
    name: &'static str,
    tests: &'static str,
}

const TABLES: [Table; 4] = [
    Table {
        name: "ts",
        tests: "src/format.test.ts",
    },
    Table {
        name: "py",
        tests: "tests/test_format.py",
    },
    Table {
        name: "rs",
        tests: "tests/format.rs",
    },
    Table {
        name: "go",
        tests: "format_test.go",
    },
];

#[test]
fn t1_stays_silent_when_the_cases_move_into_a_table() {
    for table in TABLES {
        let name = table.name;
        let repo = fixture("T1", name, "silent-table");

        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path == table.tests),
            "{name}: the test file must be in the diff, or the silence proves nothing"
        );
        let declared = cases(name, "silent-table/before", table.tests);
        let generated = entries(name, "silent-table/after", table.tests);
        assert_eq!(
            generated, declared,
            "{name}: the table has to carry every case that was written out, or the check proves nothing"
        );
        assert!(
            cases(name, "silent-table/after", table.tests) < declared,
            "{name}: the neighbour has to declare fewer cases than it runs, or a reader counting declarations sees no drop to be wrong about"
        );

        let run = repo.weed(&["check"]);
        let reported: Vec<common::Finding> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule == "T1")
            .collect();
        assert_eq!(
            reported,
            Vec::new(),
            "{name}: a case generated from a table is a case that is there"
        );
        // A table writes its assertion once and runs it per entry, which T2
        // counts as assertions dropped: that count is the tests loop's to make
        // generated-aware, and nothing else may read this change at all.
        let others: Vec<common::Finding> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule != "T2")
            .collect();
        assert_eq!(
            others,
            Vec::new(),
            "{name}: the only other reading of a suite moving into a table is what it now asserts: {others:#?}"
        );
    }
}

#[test]
fn t1_fires_when_a_table_leaves_a_case_behind() {
    for table in TABLES {
        let name = table.name;
        let repo = fixture("T1", name, "fire-table");

        let before = cases(name, "fire-table/before", table.tests);
        let after = entries(name, "fire-table/after", table.tests);
        assert!(
            after < before,
            "{name}: the table has to hold fewer entries than the file declared cases"
        );

        let run = repo.weed(&["check"]);
        let reported: Vec<common::Finding> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule == "T1")
            .collect();
        assert_eq!(
            reported.len(),
            1,
            "{name}: one finding, on the file that lost the case: {reported:#?}"
        );
        assert_eq!(reported[0].path, table.tests, "{name}: the file is named");
        assert_eq!(reported[0].level, "error", "{name}: T1 blocks");
        assert!(
            reported[0].message.contains(&before.to_string())
                && reported[0].message.contains(&after.to_string()),
            "{name}: the finding says what the count was and what it is: {}",
            reported[0].message
        );
    }
}

/// How many entries the table in a fixture's test file holds, counted the way
/// each language writes one down: one line per entry, inside the brackets the
/// table opens.
fn entries(lang: &str, case: &str, path: &str) -> usize {
    let source = fixture_file("T1", lang, case, path);
    source
        .lines()
        .map(str::trim)
        .filter(|line| match lang {
            // A record per line in the array the loop runs over.
            "ts" => line.starts_with("{ name:"),
            // A tuple per line in the list the decorator is handed.
            "py" => line.starts_with("(\"") && line.ends_with("),"),
            // A name and its two values per line of the macro invocation.
            "rs" => line.contains(" => ") && line.ends_with(','),
            // A struct literal per line of the slice the range walks.
            "go" => line.starts_with("{\"") && line.ends_with("},"),
            other => panic!("no table shape is written down for {other}"),
        })
        .count()
}

/// One case a fixture writes down: the name it carries and the lines of its
/// body. The body is the lines between this declaration and the next, without
/// the blank ones and without the punctuation that closes a block, so a case
/// that moved under another name still compares equal.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Declaration {
    name: String,
    body: Vec<String>,
}

/// The cases a fixture's test file writes, in the order it writes them, read
/// the way each language declares one. This is how the moved fixtures state
/// what they hold without a number being copied into the test.
fn declarations(lang: &str, case: &str, path: &str) -> Vec<Declaration> {
    let source = fixture_file("T1", lang, case, path);
    let lines: Vec<String> = source.lines().map(|line| line.trim().to_string()).collect();
    let declared: Vec<(String, usize)> = (0..lines.len())
        .filter_map(|index| declared(lang, &lines, index))
        .collect();
    declared
        .iter()
        .enumerate()
        .map(|(order, (name, opens))| {
            let ends = declared
                .get(order + 1)
                .map_or(lines.len(), |(_, next)| next.saturating_sub(1));
            Declaration {
                name: name.clone(),
                body: lines[*opens..ends.max(*opens)]
                    .iter()
                    .filter(|line| line.chars().any(char::is_alphanumeric))
                    .cloned()
                    .collect(),
            }
        })
        .collect()
}

/// The name the line at an index declares a case with, and the line its body
/// opens on. A language that marks a case with an attribute names it on the
/// line under that mark, so the body opens one line further down.
fn declared(lang: &str, lines: &[String], index: usize) -> Option<(String, usize)> {
    let line = lines.get(index)?;
    match lang {
        "ts" if line.starts_with("it(") => Some((quoted(line), index + 1)),
        "py" if line.starts_with("def test") => Some((between(line, "def ", "("), index + 1)),
        "rs" if line == "#[test]" => Some((between(lines.get(index + 1)?, "fn ", "("), index + 2)),
        "go" if line.starts_with("func Test") => Some((between(line, "func ", "("), index + 1)),
        "ts" | "py" | "rs" | "go" => None,
        other => panic!("no case shape is written down for {other}"),
    }
}

/// The text between the first pair of quotes on a line.
fn quoted(line: &str) -> String {
    let mut quotes = line.split('"');
    quotes.next();
    quotes
        .next()
        .unwrap_or_else(|| panic!("a case declared as a call carries a title: {line}"))
        .to_string()
}

/// The text a line writes between two markers.
fn between(line: &str, after: &str, before: &str) -> String {
    let rest = line
        .split_once(after)
        .unwrap_or_else(|| panic!("`{after}` should open the declaration: {line}"))
        .1;
    rest.split_once(before)
        .map_or(rest, |(name, _)| name)
        .to_string()
}

/// Whether a case that left one file is the same case as one that arrived in
/// another: the name it is collected under, or the body it holds where the move
/// renamed it.
fn same_case(left: &Declaration, arrived: &Declaration) -> bool {
    left.name == arrived.name || left.body == arrived.body
}

#[test]
fn t1_stays_silent_when_every_case_moved_into_another_file_in_the_diff() {
    for language in LANGUAGES {
        let name = language.name;
        let left = declarations(name, "moved/before", language.deleted);
        let held = declarations(name, "moved/before", language.thinned);
        let arrived = declarations(name, "moved/after", language.thinned);

        assert!(
            left.iter()
                .all(|case| !held.iter().any(|had| same_case(case, had))),
            "{name}: the file the cases land in must not have held them already"
        );
        let by_name: Vec<&Declaration> = left
            .iter()
            .filter(|case| arrived.iter().any(|now| now.name == case.name))
            .collect();
        let renamed: Vec<&Declaration> = left
            .iter()
            .filter(|case| !arrived.iter().any(|now| now.name == case.name))
            .collect();
        assert!(
            !by_name.is_empty(),
            "{name}: some cases have to move under their own name, or the fixture proves half the claim"
        );
        assert_eq!(
            renamed.len(),
            1,
            "{name}: exactly one case moves under another name: {renamed:#?}"
        );
        assert!(
            arrived
                .iter()
                .any(|now| now.body == renamed[0].body && now.name != renamed[0].name),
            "{name}: the renamed case has to be there under its new name, body for body: {arrived:#?}"
        );

        let repo = fixture("T1", name, "moved");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path == language.deleted),
            "{name}: the file the cases left must be in the diff, or the silence proves nothing"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a case that moved to another file in the same diff is a case that is there"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t1_counts_the_cases_that_left_the_diff_and_names_where_the_others_went() {
    for language in LANGUAGES {
        let name = language.name;
        let left = declarations(name, "moved-partly/before", language.deleted);
        let arrived = declarations(name, "moved-partly/after", language.thinned);
        let moved: Vec<&Declaration> = left
            .iter()
            .filter(|case| arrived.iter().any(|now| same_case(case, now)))
            .collect();
        let gone: Vec<&Declaration> = left
            .iter()
            .filter(|case| !arrived.iter().any(|now| same_case(case, now)))
            .collect();
        assert!(
            !moved.is_empty(),
            "{name}: some of the file's cases have to move, or the finding has nowhere to name"
        );
        assert_eq!(
            gone.len(),
            1,
            "{name}: one case has to leave the diff altogether: {gone:#?}"
        );

        let repo = fixture("T1", name, "moved-partly");
        let run = repo.weed(&["check"]);
        let reported: Vec<common::Finding> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule == "T1")
            .collect();
        assert_eq!(
            reported.len(),
            1,
            "{name}: one finding, on the file the cases left: {reported:#?}"
        );
        assert_eq!(
            reported[0].path, language.deleted,
            "{name}: the file is named"
        );
        assert_eq!(reported[0].level, "error", "{name}: T1 blocks");
        assert_eq!(run.code, 2, "{name}: a case nobody took over blocks");
        assert!(
            reported[0]
                .message
                .contains(&format!("{} test case", gone.len())),
            "{name}: the finding counts the cases that left and not the ones that moved: {}",
            reported[0].message
        );
        assert!(
            reported[0]
                .message
                .contains(&format!("{} test cases", moved.len())),
            "{name}: the finding says how many moved: {}",
            reported[0].message
        );
        assert!(
            reported[0].message.contains(language.thinned),
            "{name}: the finding names the file they moved into: {}",
            reported[0].message
        );
        assert!(
            !reported[0].message.contains(COVERED_BY_NOBODY),
            "{name}: a case another file in the diff took over is covered by somebody: {}",
            reported[0].message
        );
    }
}

/// The sentence T1 may only write when nothing in the diff took the cases over.
const COVERED_BY_NOBODY: &str = "covered by nobody";
