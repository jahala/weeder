//! T1, a test was deleted.
//!
//! Two shapes, in four languages. A test file removed takes every case it held;
//! a test file that stayed and declares fewer cases lost the difference. The
//! neighbour does the two honest things that look the same from a distance, a
//! case renamed where it stands, and a case added, and weeder says nothing.
//!
//! A third pair reads the move a growing suite makes: the cases stop being
//! written out one at a time and come from a table instead, a `for` around an
//! `it`, a `t.Run` in a range, a `parametrize` list, a macro invoked once per
//! line. Nothing is covered less afterwards, so weeder counts the entries and
//! says nothing; a table that leaves a case behind is still a case gone.
//!
//! A fourth pair reads the move a suite makes between files. Splitting one file
//! into two, or folding two into one, takes cases out of one file and puts them
//! into another in the same diff, some under their own name and some renamed on
//! the way, and nothing is covered less afterwards. A case that went nowhere is
//! a case gone like any other, and the finding that reports it counts only what
//! left and names the files the rest moved into.
//!
//! Two more read the moves that only look like one. A case that arrives under
//! its old name making fewer assertions is a case written over, and counts as
//! gone. A test file moved into a `[scope] exclude` directory leaves the
//! tree weeder judges, and is judged as deleted from the path it left.
//!
//! The last four read a function that moves from one file to another while
//! its tests follow it, renamed and rewritten, so neither a name nor a body is
//! left to follow them by. T1 and T2 both read the one change, so these
//! fixtures are asked about both. Where the case that arrives calls the moved
//! function and asserts as much as the three that left, nothing went. Where it
//! asserts less, both still block, and T2 names the shortfall. Where the tests
//! move and no code does, both block as before, and so does T1 where the cases
//! that left asserted nothing, since no count holds an arrival to them.
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
        let run = repo.weeder(&["check"]);
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

        let run = repo.weeder(&["check"]);
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

        let run = repo.weeder(&["check"]);
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

        let run = repo.weeder(&["check"]);
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

        let run = repo.weeder(&["check"]);
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
        let run = repo.weeder(&["check"]);
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

/// Whether a line of a fixture's test file makes an assertion, read the way each
/// language makes one. This is how the hollowed fixture shows that the case it
/// keeps by name checks less than the case that left.
fn asserts(lang: &str, line: &str) -> bool {
    match lang {
        "ts" => line.contains("expect("),
        "py" | "rs" => line.starts_with("assert"),
        "go" => line.contains("t.Fatal") || line.contains("t.Error"),
        other => panic!("no assertion shape is written down for {other}"),
    }
}

fn assertions(lang: &str, declaration: &Declaration) -> usize {
    declaration
        .body
        .iter()
        .filter(|line| asserts(lang, line))
        .count()
}

#[test]
fn t1_fires_when_a_case_moves_under_its_own_name_holding_fewer_assertions() {
    for language in LANGUAGES {
        let name = language.name;
        let left = declarations(name, "moved-hollow/before", language.deleted);
        let arrived = declarations(name, "moved-hollow/after", language.thinned);
        let hollowed: Vec<(&Declaration, &Declaration)> = left
            .iter()
            .filter_map(|case| {
                arrived
                    .iter()
                    .find(|now| now.name == case.name)
                    .filter(|now| assertions(name, now) < assertions(name, case))
                    .map(|now| (case, now))
            })
            .collect();
        assert_eq!(
            hollowed.len(),
            1,
            "{name}: exactly one case arrives under its own name making fewer assertions: {arrived:#?}"
        );
        let kept = left
            .iter()
            .filter(|case| {
                arrived
                    .iter()
                    .any(|now| now == *case || (now.name == case.name && now.body == case.body))
            })
            .count();
        assert_eq!(
            kept,
            left.len() - 1,
            "{name}: every other case moves whole, so the finding counts one"
        );

        let repo = fixture("T1", name, "moved-hollow");
        let run = repo.weeder(&["check"]);
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
        assert_eq!(run.code, 2, "{name}: a case written over with less blocks");
        assert!(
            reported[0].message.contains("1 test case went"),
            "{name}: the hollowed case is the one that went: {}",
            reported[0].message
        );
        assert!(
            reported[0]
                .message
                .contains(&format!("{} test cases moved", kept)),
            "{name}: the cases that moved whole are counted as moved: {}",
            reported[0].message
        );
    }
}

/// Where the specimen fixture moves the test file to.
fn in_specimens(path: &str) -> String {
    format!("fixtures/adversarial/parser/{path}")
}

#[test]
fn t1_judges_a_test_moved_into_a_specimen_directory_as_deleted() {
    for language in LANGUAGES {
        let name = language.name;
        let repo = fixture("T1", name, "specimen");
        let moved = in_specimens(language.deleted);
        let status = repo.git(&["diff", "--cached", "--find-renames", "--name-status"]);
        assert!(
            status
                .lines()
                .any(|line| line.starts_with('R') && line.ends_with(&moved)),
            "{name}: git has to see a rename into the specimen directory, or the test proves nothing:\n{status}"
        );

        let run = repo.weeder(&["check", "--format", "sarif"]);
        let findings = run.findings();
        let t1: Vec<&common::Finding> = findings.iter().filter(|f| f.rule == "T1").collect();
        assert_eq!(
            t1.len(),
            1,
            "{name}: the suite lost a file, whatever directory it went to: {findings:#?}"
        );
        assert_eq!(
            t1[0].path, language.deleted,
            "{name}: the path it left is named"
        );
        assert_eq!(t1[0].level, "error", "{name}: T1 blocks");
        assert_eq!(run.code, 2, "{name}: {}", run.stderr);
        assert!(
            findings.iter().any(|finding| finding.rule == "EXCLUDED"
                && finding.path == "fixtures/adversarial/"
                && finding.message.contains("1 file")),
            "{name}: where it went is noted as excluded, counted under the pattern: {findings:#?}"
        );
    }
}

#[test]
fn a_file_moved_from_one_specimen_directory_to_another_stays_set_aside() {
    let repo = common::Repo::init();
    repo.write(
        "weeder.toml",
        "[scope]\nexclude = [\"fixtures/adversarial/\"]\n",
    );
    let suite = fixture_file("T1", "ts", "moved/before", "src/parser.test.ts");
    repo.write("fixtures/adversarial/one/parser.test.ts", &suite);
    repo.commit("a specimen");
    std::fs::create_dir_all(repo.root().join("fixtures/adversarial/two"))
        .expect("the directory it moves to");
    repo.git(&[
        "mv",
        "fixtures/adversarial/one/parser.test.ts",
        "fixtures/adversarial/two/parser.test.ts",
    ]);

    let run = repo.weeder(&["check", "--format", "sarif"]);
    let rules: Vec<String> = run
        .findings()
        .into_iter()
        .map(|finding| finding.rule)
        .collect();
    assert_eq!(
        rules,
        vec!["EXCLUDED".to_string()],
        "a specimen that stays a specimen is noted and judged by nothing: {}",
        run.output()
    );
    assert_eq!(run.code, 0);
}

/// Each language's skipped fixture: the suite whose cases are turned off,
/// focused or left to do rather than deleted, and the marks it writes to do so.
const SKIPPED: [(&str, &str, &[&str]); 4] = [
    (
        "ts",
        "src/format.test.ts",
        &[".skip(", ".only(", "xit(", ".todo("],
    ),
    ("py", "tests/test_format.py", &["@pytest.mark."]),
    ("rs", "tests/format.rs", &["#[ignore"]),
    ("go", "format_test.go", &["t.Skip"]),
];

#[test]
fn a_case_turned_skip_focus_or_todo_is_t3_s_finding_and_not_a_case_gone() {
    for (name, path, marks) in SKIPPED {
        let source = fixture_file("T1", name, "skipped/after", path);
        let marked: Vec<u64> = source
            .lines()
            .enumerate()
            .filter(|(_, line)| marks.iter().any(|mark| line.contains(mark)))
            .map(|(index, _)| index as u64 + 1)
            .collect();
        assert!(
            marked.len() >= 2,
            "{name}: the fixture turns cases off, or the silence proves nothing"
        );

        let run = fixture("T1", name, "skipped").weeder(&["check"]);
        let findings = run.findings();
        let deleted: Vec<_> = findings
            .iter()
            .filter(|finding| finding.rule == "T1")
            .collect();
        assert!(
            deleted.is_empty(),
            "{name}: a case turned off is still declared, so nothing disappeared: {deleted:#?}"
        );
        let skipped: Vec<u64> = findings
            .iter()
            .filter(|finding| finding.rule == "T3")
            .filter_map(|finding| finding.line)
            .collect();
        assert_eq!(
            skipped, marked,
            "{name}: T3 reports every mark, which is the finding the change earned"
        );
        assert_eq!(run.code, 2, "{name}: and it blocks");
    }
}

/// Each language's suite of contrast cases, written out one at a time before
/// the change and generated afterwards by tables inside a table: a `describe`
/// handed the themes and an `each` per kind of pair inside it, a class handed
/// the themes and its tests handed their pairs, stacked, a subtest per theme
/// running a subtest per pair, and a row of cases crossed with a row of values.
/// Some of the tables are consts the file declares, one is spread from another,
/// and one is mapped on its way in.
const NESTED: [Table; 4] = [
    Table {
        name: "ts",
        tests: "src/contrast.test.ts",
    },
    Table {
        name: "py",
        tests: "tests/test_contrast.py",
    },
    Table {
        name: "rs",
        tests: "tests/contrast.rs",
    },
    Table {
        name: "go",
        tests: "contrast_test.go",
    },
];

/// The pair the fire variant takes out of the inner table, which every theme
/// of the outer table ran.
const DROPPED: &str = "muted";

#[test]
fn t1_and_t2_stay_silent_when_written_out_cases_become_tables_inside_a_table() {
    for table in NESTED {
        let name = table.name;
        let declared = cases(name, "silent-nested/before", table.tests);
        assert!(
            declared > 4,
            "{name}: the suite has to write out a case per theme and pair, or the check proves nothing"
        );
        assert!(
            cases(name, "silent-nested/after", table.tests) < declared,
            "{name}: the tables have to declare fewer cases than they run, or a reader counting declarations sees no drop to be wrong about"
        );

        let repo = fixture("T1", name, "silent-nested");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path == table.tests),
            "{name}: the test file must be in the diff, or the silence proves nothing"
        );
        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: every case still runs once for every theme, and so does every claim"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t1_and_t2_count_a_row_an_inner_table_lost_once_for_every_row_of_the_outer_one() {
    for table in NESTED {
        let name = table.name;
        let written = declarations(name, "fire-nested/before", table.tests);
        let before = written.len();
        let claims: usize = written.iter().map(|case| assertions(name, case)).sum();
        let lost: Vec<&Declaration> = written
            .iter()
            .filter(|case| case.name.to_lowercase().contains(DROPPED))
            .collect();
        assert!(
            lost.len() > 1,
            "{name}: the dropped pair has to run under more than one theme, or the outer table multiplies nothing"
        );
        let gone = lost.len();
        let unmade: usize = lost.iter().map(|case| assertions(name, case)).sum();

        let run = fixture("T1", name, "fire-nested").weeder(&["check"]);
        let findings = run.findings();
        let t1: Vec<&common::Finding> = findings.iter().filter(|f| f.rule == "T1").collect();
        assert_eq!(t1.len(), 1, "{name}: one T1 finding: {findings:#?}");
        assert_eq!(t1[0].level, "error", "{name}: T1 blocks");
        assert!(
            t1[0].message.contains(&format!(
                "{gone} test cases disappeared from this file: it declared {before} and now declares {}.",
                before - gone
            )),
            "{name}: the tables are counted row by row, each inner row once per outer row: {}",
            t1[0].message
        );
        let t2: Vec<&common::Finding> = findings.iter().filter(|f| f.rule == "T2").collect();
        assert_eq!(t2.len(), 1, "{name}: one T2 finding: {findings:#?}");
        assert_eq!(t2[0].level, "error", "{name}: T2 blocks");
        assert!(
            t2[0].message.contains(&format!(
                "{unmade} assertions went out of this file: it made {claims} and now makes {}.",
                claims - unmade
            )),
            "{name}: a claim runs once for every row of every table around it: {}",
            t2[0].message
        );
        assert_eq!(run.code, 2, "{name}: the change is stopped");
    }
}

/// The languages whose tests can be generated from a table weeder cannot
/// count: an `each` handed a table another module exports, a decorator handed
/// one, a subtest per entry of what a function returns. A Rust test is handed
/// its rows by attributes and macros, which write every row down.
const UNCOUNTED: [Table; 3] = [
    Table {
        name: "ts",
        tests: "src/format.test.ts",
    },
    Table {
        name: "py",
        tests: "tests/test_format.py",
    },
    Table {
        name: "go",
        tests: "format_test.go",
    },
];

#[test]
fn t1_warns_when_the_cases_that_left_now_come_from_a_table_it_cannot_count() {
    for table in UNCOUNTED {
        let name = table.name;
        let before = cases(name, "uncounted/before", table.tests);
        assert!(
            before > 1,
            "{name}: the suite has to write out more than one case, or the warning proves nothing"
        );
        assert!(
            cases(name, "uncounted/after", table.tests) <= 1,
            "{name}: every case written out has to go but the one the table drives, so only the table can hold them"
        );

        let run = fixture("T1", name, "uncounted").weeder(&["check"]);
        let findings = run.findings();
        let t1: Vec<&common::Finding> = findings.iter().filter(|f| f.rule == "T1").collect();
        assert_eq!(t1.len(), 1, "{name}: one T1 finding: {findings:#?}");
        assert_eq!(
            t1[0].level, "warning",
            "{name}: a loss weeder cannot prove is a warning for the person, not a block: {}",
            t1[0].message
        );
        assert!(
            t1[0]
                .message
                .contains(&format!("it declared {before} and now declares 1"))
                && t1[0].message.contains("cannot count"),
            "{name}: the finding names both counts and why it cannot decide: {}",
            t1[0].message
        );
        assert!(
            !t1[0].message.contains("unwatched"),
            "{name}: the table may still hold every case, so the finding may not say they are unwatched: {}",
            t1[0].message
        );
        assert_eq!(run.code, 0, "{name}: a warning stops nobody: {findings:#?}");
    }
}

#[test]
fn t1_still_blocks_a_case_gone_beside_a_table_it_cannot_count_that_did_not_change() {
    for table in UNCOUNTED {
        let name = table.name;
        let before = cases(name, "uncounted-kept/before", table.tests);
        let after = cases(name, "uncounted-kept/after", table.tests);
        assert_eq!(
            after + 1,
            before,
            "{name}: one written-out case goes, and the table stays as it was"
        );

        let run = fixture("T1", name, "uncounted-kept").weeder(&["check"]);
        let findings = run.findings();
        let t1: Vec<&common::Finding> = findings.iter().filter(|f| f.rule == "T1").collect();
        assert_eq!(t1.len(), 1, "{name}: one T1 finding: {findings:#?}");
        assert_eq!(t1[0].level, "error", "{name}: T1 blocks");
        assert!(
            t1[0].message.contains("1 test case disappeared"),
            "{name}: the table was there on both sides, so the case that went is provably gone: {}",
            t1[0].message
        );
        assert_eq!(run.code, 2, "{name}: the change is stopped");
    }
}

#[test]
fn a_deleted_test_file_whose_cases_weeder_cannot_count_is_a_warning_not_silence() {
    // A bats suite's cases are not read yet (jahala/tilth#318), so the file
    // counts as no cases at all. Deleting it took a suite out of the run all
    // the same, and saying nothing about that is the one wrong answer. weeder
    // cannot say how much went, so it warns rather than blocks: a person reads
    // whether the file held tests or only helped them.
    let repo = common::Repo::init();
    repo.write(
        "test/check.bats",
        "@test \"the gate refuses an empty input\" {\n  run ./check.sh \"\"\n  [ \"$status\" -eq 1 ]\n}\n",
    );
    repo.commit("a suite the change starts from");
    std::fs::remove_file(repo.root().join("test/check.bats")).expect("the suite is deleted");

    let run = repo.weeder(&["check"]);
    let t1: Vec<common::Finding> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == "T1")
        .collect();
    assert_eq!(
        t1.len(),
        1,
        "the deleted suite is reported: {t1:#?}\n{}",
        run.stderr
    );
    assert_eq!(
        t1[0].level, "warning",
        "an uncounted fall is read by a person"
    );
    assert_eq!(t1[0].path, "test/check.bats");
    assert!(
        t1[0].message.contains("could not count"),
        "the finding says why it cannot say how much went: {}",
        t1[0].message
    );
    assert_eq!(run.code, 0, "a warning stops nobody");
}

/// A language's half of the fixtures where a function moves from one file to
/// another and its tests follow it: the test file the cases leave, the one a
/// case arrives in, the production files the function leaves and arrives in,
/// and the line that defines it.
struct Followed {
    name: &'static str,
    left: &'static str,
    into: &'static str,
    home: &'static str,
    new_home: &'static str,
    defines: &'static str,
}

const FOLLOWED: [Followed; 4] = [
    Followed {
        name: "ts",
        left: "src/manager.test.ts",
        into: "src/client.test.ts",
        home: "src/manager.ts",
        new_home: "src/client.ts",
        defines: "export function retryConnect(",
    },
    Followed {
        name: "py",
        left: "tests/test_manager.py",
        into: "tests/test_client.py",
        home: "src/manager.py",
        new_home: "src/client.py",
        defines: "def retry_connect(",
    },
    Followed {
        name: "rs",
        left: "tests/manager.rs",
        into: "tests/client.rs",
        home: "src/manager.rs",
        new_home: "src/client.rs",
        defines: "pub fn retry_connect(",
    },
    Followed {
        name: "go",
        left: "manager_test.go",
        into: "client_test.go",
        home: "manager.go",
        new_home: "client.go",
        defines: "func retryConnect(",
    },
];

/// How many assertions a fixture's file makes, read the way each language
/// makes one.
fn assertions_in(lang: &str, case: &str, path: &str) -> usize {
    fixture_file("T1", lang, case, path)
        .lines()
        .map(str::trim)
        .filter(|line| asserts(lang, line))
        .count()
}

/// What one of the followed fixtures does, as this file reads it: how many
/// cases left the first test file, the assertions they made, and the
/// assertions the cases that arrived in the second one make. No case that
/// left may be followed by its name or its body, or the move is one weeder
/// already read and the fixture proves nothing new.
struct Move {
    cases: usize,
    left: usize,
    arrived: usize,
}

fn followed_move(language: &Followed, state: &str) -> Move {
    let name = language.name;
    let before = format!("{state}/before");
    let after = format!("{state}/after");
    let kept = declarations(name, &after, language.left);
    let gone: Vec<Declaration> = declarations(name, &before, language.left)
        .into_iter()
        .filter(|case| !kept.iter().any(|now| now.name == case.name))
        .collect();
    let arrived = declarations(name, &after, language.into);
    assert!(
        gone.iter()
            .all(|case| !arrived.iter().any(|now| same_case(case, now))),
        "{name}/{state}: no case may follow by its name or its body: {arrived:#?}"
    );
    Move {
        cases: gone.len(),
        left: assertions_in(name, &before, language.left)
            - assertions_in(name, &after, language.left),
        arrived: assertions_in(name, &after, language.into)
            - assertions_in(name, &before, language.into),
    }
}

/// Whether a fixture's production file defines the function.
fn defines(language: &Followed, side: &str, path: &str) -> bool {
    fixture_file("T1", language.name, side, path).contains(language.defines)
}

/// The T1 and the T2 findings of a run, each apart.
fn by_rule(run: &common::Run) -> (Vec<common::Finding>, Vec<common::Finding>) {
    let findings = run.findings();
    let of = |rule: &str| -> Vec<common::Finding> {
        findings
            .iter()
            .filter(|finding| finding.rule == rule)
            .cloned()
            .collect()
    };
    (of("T1"), of("T2"))
}

#[test]
fn t1_and_t2_read_cases_that_followed_their_function_into_another_file_as_moved() {
    for language in FOLLOWED {
        let name = language.name;
        assert!(
            defines(&language, "followed/before", language.home)
                && !defines(&language, "followed/after", language.home)
                && defines(&language, "followed/after", language.new_home),
            "{name}: the function has to move from one production file to another"
        );
        let moved = followed_move(&language, "followed");
        assert_eq!(moved.cases, 3, "{name}: three cases leave the file");
        assert!(
            moved.arrived >= moved.left,
            "{name}: the case that arrived has to assert at least what the three did: {} against {}",
            moved.arrived,
            moved.left
        );

        let run = fixture("T1", name, "followed").weeder(&["check"]);
        let (t1, t2) = by_rule(&run);
        assert_eq!(
            t1,
            Vec::new(),
            "{name}: the cases followed their function, renamed and rewritten"
        );
        assert_eq!(
            t2,
            Vec::new(),
            "{name}: the assertions followed their function, rewritten"
        );
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a function moved with its tests is nothing to report"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t1_and_t2_block_when_the_case_that_followed_the_function_asserts_less() {
    for language in FOLLOWED {
        let name = language.name;
        let moved = followed_move(&language, "followed-short");
        assert!(
            moved.arrived > 0 && moved.arrived < moved.left,
            "{name}: the case that arrives has to assert something, and less than what left"
        );
        assert!(
            defines(&language, "followed-short/after", language.new_home),
            "{name}: the function still moves"
        );

        let run = fixture("T1", name, "followed-short").weeder(&["check"]);
        let (t1, t2) = by_rule(&run);
        assert_eq!(run.code, 2, "{name}: a move that checks less blocks");
        assert_eq!(t1.len(), 1, "{name}: one T1 finding: {t1:#?}");
        assert_eq!(t1[0].path, language.left, "{name}: on the file they left");
        assert_eq!(t1[0].level, "error", "{name}: T1 blocks");
        assert!(
            t1[0]
                .message
                .contains(&format!("{} test cases disappeared", moved.cases)),
            "{name}: no case is paid for by assertions it does not make: {}",
            t1[0].message
        );
        assert_eq!(t2.len(), 1, "{name}: one T2 finding: {t2:#?}");
        assert_eq!(t2[0].path, language.left, "{name}: on the file they left");
        assert_eq!(t2[0].level, "error", "{name}: T2 blocks");
        let short = moved.left - moved.arrived;
        assert!(
            t2[0]
                .message
                .contains(&format!("{short} assertions went out of this file")),
            "{name}: the finding counts the shortfall: {}",
            t2[0].message
        );
        assert!(
            t2[0].message.contains(&format!(
                "{} assertion moved to `{}`",
                moved.arrived, language.into
            )),
            "{name}: the finding names the file the rest went to: {}",
            t2[0].message
        );
    }
}

#[test]
fn t1_and_t2_block_when_tests_leave_a_file_and_no_function_moved() {
    for language in FOLLOWED {
        let name = language.name;
        let moved = followed_move(&language, "unmoved");
        assert!(
            moved.arrived >= moved.left,
            "{name}: the case that arrives asserts as much, so only the code tells this apart"
        );
        let repo = fixture("T1", name, "unmoved");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            !changed
                .lines()
                .any(|path| path == language.home || path == language.new_home),
            "{name}: no production file is in the change: {changed}"
        );

        let run = repo.weeder(&["check"]);
        let (t1, t2) = by_rule(&run);
        assert_eq!(run.code, 2, "{name}: tests gone with no code moved block");
        assert_eq!(t1.len(), 1, "{name}: one T1 finding: {t1:#?}");
        assert_eq!(t1[0].path, language.left, "{name}: on the file they left");
        assert_eq!(t1[0].level, "error", "{name}: T1 blocks");
        assert!(
            t1[0]
                .message
                .contains(&format!("{} test cases disappeared", moved.cases)),
            "{name}: every case that left is counted: {}",
            t1[0].message
        );
        assert_eq!(t2.len(), 1, "{name}: one T2 finding: {t2:#?}");
        assert_eq!(t2[0].path, language.left, "{name}: on the file they left");
        assert_eq!(t2[0].level, "error", "{name}: T2 blocks");
        assert!(
            t2[0]
                .message
                .contains(&format!("{} assertions went out of this file:", moved.left)),
            "{name}: every assertion that left is counted: {}",
            t2[0].message
        );
    }
}

#[test]
fn t1_blocks_when_cases_that_asserted_nothing_follow_their_function() {
    for language in FOLLOWED {
        let name = language.name;
        let moved = followed_move(&language, "followed-hollow");
        assert_eq!(
            moved.left, 0,
            "{name}: the cases that leave assert nothing, so no count can hold an arrival to them"
        );
        assert!(
            moved.cases > 1 && moved.arrived > 0,
            "{name}: several cases leave, and the case that arrives asserts, so a ride on it would pass"
        );
        assert!(
            defines(&language, "followed-hollow/after", language.new_home),
            "{name}: the function still moves"
        );

        let run = fixture("T1", name, "followed-hollow").weeder(&["check"]);
        let (t1, t2) = by_rule(&run);
        assert_eq!(
            run.code, 2,
            "{name}: cases that asserted nothing still block when they go"
        );
        assert_eq!(t1.len(), 1, "{name}: one T1 finding: {t1:#?}");
        assert_eq!(t1[0].path, language.left, "{name}: on the file they left");
        assert_eq!(t1[0].level, "error", "{name}: T1 blocks");
        assert!(
            t1[0]
                .message
                .contains(&format!("{} test cases disappeared", moved.cases)),
            "{name}: every case that left is counted: {}",
            t1[0].message
        );
        assert_eq!(
            t2,
            Vec::new(),
            "{name}: no assertion left, so T2 has nothing to say"
        );
    }
}
