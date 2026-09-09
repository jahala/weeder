//! T2, assertions were dropped from a changed test file.
//!
//! The counts the finding has to name are read off the fixture here, by looking
//! for the forms each language's frameworks write an assertion in, so the
//! expectation does not depend on how the detector goes about counting them.
//!
//! The neighbour moves an assertion from one case to another and changes
//! nothing else: the same claims, made in a different place. The allowance is
//! the fire fixture again, this time with a commit message that says why the
//! suite is making fewer claims than it was.
//!
//! A second pair reads the move a suite makes between files. A case goes to a
//! sibling test file in the same change and takes its claims with it, one of
//! them rewritten on the way, and one more claim lands there under a case that
//! was renamed: nothing is claimed less afterwards. A claim that went nowhere
//! is a claim dropped like any other, and the finding that reports it counts
//! only what left and names the file the rest moved into.

mod common;

use common::{fixture, fixture_file};

/// A language, the test file its assertions live in, the sibling file the
/// moved fixtures send them to, and the way that language writes one.
struct Language {
    name: &'static str,
    path: &'static str,
    into: &'static str,
    forms: &'static [&'static str],
}

const LANGUAGES: [Language; 4] = [
    Language {
        name: "ts",
        path: "src/format.test.ts",
        into: "src/padding.test.ts",
        forms: &["expect("],
    },
    Language {
        name: "py",
        path: "tests/test_format.py",
        into: "tests/test_padding.py",
        forms: &["assert "],
    },
    Language {
        name: "rs",
        path: "tests/format.rs",
        into: "tests/padding.rs",
        forms: &["assert_eq!("],
    },
    Language {
        name: "go",
        path: "format_test.go",
        into: "padding_test.go",
        forms: &["t.Errorf("],
    },
];

/// The reason a change carries when it means to make fewer claims.
const REASON: &str = "the two padding cases became one, and the second claim moved into it";

#[test]
fn t2_fires_at_block_level_when_a_changed_test_file_makes_fewer_claims() {
    for language in LANGUAGES {
        let repo = fixture("T2", language.name, "fire");
        let name = language.name;
        let before = assertions(&language, "fire/before");
        let after = assertions(&language, "fire/after");
        assert!(
            after < before && after > 0,
            "{name}: the fixture has to drop assertions and keep some, not empty the file"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.code, 2,
            "{name}: dropped assertions block\n{}",
            run.stderr
        );

        let findings: Vec<common::Finding> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule == "T2")
            .collect();
        assert_eq!(
            findings.len(),
            1,
            "{name}: one file, one finding: {findings:?}"
        );
        let finding = &findings[0];
        assert_eq!(finding.level, "error", "{name}: T2 blocks");
        assert_eq!(finding.path, language.path, "{name}: the file is named");
        assert!(
            finding.message.contains(&before.to_string())
                && finding.message.contains(&after.to_string()),
            "{name}: the finding names both counts: {}",
            finding.message
        );
        assert!(
            finding.message.contains(HELD_BY_NOBODY),
            "{name}: nothing in this diff took the claims over, so the finding may say so: {}",
            finding.message
        );
    }
}

#[test]
fn t2_stays_silent_when_the_same_claims_are_made_from_another_case() {
    for language in LANGUAGES {
        let repo = fixture("T2", language.name, "silent");
        let name = language.name;

        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path == language.path),
            "{name}: the neighbour must be in the diff, or the silence proves nothing"
        );
        assert_eq!(
            assertions(&language, "silent/before"),
            assertions(&language, "silent/after"),
            "{name}: the neighbour moves an assertion and drops none"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: an assertion that moved is an assertion that is still made"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t2_is_allowed_by_a_trailer_that_carries_a_reason() {
    for language in LANGUAGES {
        let repo = fixture("T2", language.name, "fire");
        let name = language.name;
        repo.pending_message(&format!(
            "Fold the padding cases together\n\nWeeder-allow: T2 {REASON}\n"
        ));

        let run = repo.weeder(&["check"]);
        let findings = run.findings();
        assert!(
            findings.iter().any(|finding| finding.rule == "T2"),
            "{name}: an allowed finding is still reported"
        );
        assert!(
            findings
                .iter()
                .filter(|finding| finding.rule == "T2")
                .all(|finding| finding.suppressed && finding.level == "note"),
            "{name}: the trailer stands the finding down: {findings:?}"
        );
        assert_eq!(
            run.code, 0,
            "{name}: an allowance a reason travels with stops nobody"
        );
    }
}

#[test]
fn t2_refuses_a_trailer_with_no_reason() {
    let language = &LANGUAGES[0];
    let repo = fixture("T2", language.name, "fire");
    repo.pending_message("Fold the padding cases together\n\nWeeder-allow: T2\n");

    let run = repo.weeder(&["check"]);
    assert!(
        run.findings()
            .iter()
            .any(|finding| finding.rule == "T2" && !finding.suppressed),
        "a trailer that says nothing allows nothing"
    );
    assert_eq!(run.code, 2, "and the change is still stopped");
}

/// How many assertions a fixture's test file makes, counted from the forms the
/// language writes one in.
fn assertions(language: &Language, case: &str) -> usize {
    let (state, side) = case
        .split_once('/')
        .expect("a fixture case is written as `case/side`");
    let source = fixture_file(
        "T2",
        language.name,
        state,
        &format!("{side}/{}", language.path),
    );
    source
        .lines()
        .map(|line| {
            language
                .forms
                .iter()
                .map(|form| line.matches(form).count())
                .sum::<usize>()
        })
        .sum()
}

/// One claim a fixture's test file makes: the case it sits in, and the line it
/// is written on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Claim {
    case: String,
    line: String,
}

/// The claims a fixture's test file makes, in the order it makes them, read the
/// way each language declares a case and writes an assertion. This is how the
/// moved fixtures state what left and what arrived without a number being
/// copied into the test.
fn claims(language: &Language, case: &str, path: &str) -> Vec<Claim> {
    let (state, side) = case
        .split_once('/')
        .expect("a fixture case is written as `case/side`");
    let source = fixture_file("T2", language.name, state, &format!("{side}/{path}"));
    let lines: Vec<String> = source.lines().map(|line| line.trim().to_string()).collect();
    let mut current = String::new();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if let Some(name) = declared(language.name, &lines, index) {
            current = name;
        }
        let made: usize = language
            .forms
            .iter()
            .map(|form| line.matches(form).count())
            .sum();
        for _ in 0..made {
            assert!(
                !current.is_empty(),
                "{}: a claim outside every case: {line}",
                language.name
            );
            found.push(Claim {
                case: current.clone(),
                line: line.clone(),
            });
        }
    }
    found
}

/// The name the line at an index declares a case with, where it declares one. A
/// language that marks a case with an attribute names it on the line under that
/// mark, and the name is read from there.
fn declared(lang: &str, lines: &[String], index: usize) -> Option<String> {
    let line = lines.get(index)?;
    match lang {
        "ts" if line.starts_with("it(") => Some(quoted(line)),
        "py" if line.starts_with("def test") => Some(between(line, "def ", "(")),
        "rs" if line == "#[test]" => Some(between(lines.get(index + 1)?, "fn ", "(")),
        "go" if line.starts_with("func Test") => Some(between(line, "func ", "(")),
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

/// Whether a claim made elsewhere answers for one that left: the same line,
/// wherever it is now written, or the same case, however the line inside it was
/// rewritten on the way.
fn answers(left: &Claim, made: &Claim) -> bool {
    left.line == made.line || left.case == made.case
}

/// The claims on the left that nothing on the right answers for, each answer
/// spoken for once. A line written the same way is paired first, so a rewritten
/// one is not credited with a partner an identical one has the better claim to.
fn unanswered(left: &[Claim], right: &[Claim]) -> Vec<Claim> {
    let mut taken = vec![false; right.len()];
    let mut paired = vec![false; left.len()];
    for same_line in [true, false] {
        for (index, claim) in left.iter().enumerate() {
            if paired[index] {
                continue;
            }
            let found = right.iter().enumerate().find(|(at, other)| {
                !taken[*at]
                    && if same_line {
                        claim.line == other.line
                    } else {
                        answers(claim, other)
                    }
            });
            if let Some((at, _)) = found {
                taken[at] = true;
                paired[index] = true;
            }
        }
    }
    left.iter()
        .enumerate()
        .filter(|(index, _)| !paired[*index])
        .map(|(_, claim)| claim.clone())
        .collect()
}

/// What one of the moved fixtures does with the claims that left its test file:
/// what arrived in the sibling file, and what arrived nowhere at all.
struct Move {
    lost: Vec<Claim>,
    arrived: Vec<Claim>,
    stranded: Vec<Claim>,
}

impl Move {
    /// How many of the claims that left the file another file took over.
    fn moved(&self) -> usize {
        self.lost.len() - self.stranded.len()
    }
}

fn followed(language: &Language, state: &str) -> Move {
    let before = claims(language, &format!("{state}/before"), language.path);
    let after = claims(language, &format!("{state}/after"), language.path);
    let lost = unanswered(&before, &after);
    let held = claims(language, &format!("{state}/before"), language.into);
    let now = claims(language, &format!("{state}/after"), language.into);
    let arrived = unanswered(&now, &held);
    let stranded = unanswered(&lost, &arrived);
    Move {
        lost,
        arrived,
        stranded,
    }
}

#[test]
fn t2_stays_silent_when_every_dropped_claim_moved_into_another_file_in_the_diff() {
    for language in LANGUAGES {
        let name = language.name;
        let followed = followed(&language, "moved");
        assert!(
            assertions(&language, "moved/after") < assertions(&language, "moved/before"),
            "{name}: the file has to make fewer claims than it did, or the silence proves nothing"
        );
        assert!(
            followed.lost.len() > 2,
            "{name}: more than one claim has to leave the file: {:#?}",
            followed.lost
        );
        assert!(
            followed.stranded.is_empty(),
            "{name}: every claim that left has to arrive in the sibling file: {:#?}",
            followed.stranded
        );

        let renamed: Vec<&Claim> = followed
            .lost
            .iter()
            .filter(|claim| {
                followed
                    .arrived
                    .iter()
                    .any(|made| made.line == claim.line && made.case != claim.case)
            })
            .collect();
        assert!(
            !renamed.is_empty(),
            "{name}: one claim has to arrive under a case renamed on the way, so the line is what follows it"
        );
        let rewritten: Vec<&Claim> = followed
            .lost
            .iter()
            .filter(|claim| !followed.arrived.iter().any(|made| made.line == claim.line))
            .collect();
        assert!(
            !rewritten.is_empty(),
            "{name}: one claim has to arrive rewritten, so the case it sat in is what follows it"
        );
        assert!(
            rewritten
                .iter()
                .all(|claim| followed.arrived.iter().any(|made| made.case == claim.case)),
            "{name}: the rewritten claim has to land in a case of the same name: {rewritten:#?}"
        );

        let repo = fixture("T2", name, "moved");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|path| path == language.path),
            "{name}: the file the claims left must be in the diff, or the silence proves nothing"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a claim that moved to another file in the same diff is a claim still made"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

#[test]
fn t2_counts_the_claims_that_left_the_diff_and_names_where_the_others_went() {
    for language in LANGUAGES {
        let name = language.name;
        let followed = followed(&language, "moved-partly");
        assert_eq!(
            followed.stranded.len(),
            1,
            "{name}: one claim has to arrive nowhere: {:#?}",
            followed.stranded
        );
        assert!(
            followed.moved() > 1,
            "{name}: the rest have to move, or the finding has nothing to name"
        );

        let repo = fixture("T2", name, "moved-partly");
        let run = repo.weeder(&["check"]);
        let reported: Vec<common::Finding> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule == "T2")
            .collect();
        assert_eq!(
            reported.len(),
            1,
            "{name}: one finding, on the file the claims left: {reported:#?}"
        );
        assert_eq!(reported[0].path, language.path, "{name}: the file is named");
        assert_eq!(reported[0].level, "error", "{name}: T2 blocks");
        assert_eq!(run.code, 2, "{name}: a claim nobody took over blocks");
        assert!(
            reported[0]
                .message
                .contains(&format!("{} assertion ", followed.stranded.len())),
            "{name}: the finding counts the claims that left and not the ones that moved: {}",
            reported[0].message
        );
        assert!(
            reported[0]
                .message
                .contains(&format!("{} assertions ", followed.moved())),
            "{name}: the finding says how many moved: {}",
            reported[0].message
        );
        assert!(
            reported[0].message.contains(language.into),
            "{name}: the finding names the file they moved into: {}",
            reported[0].message
        );
        assert!(
            !reported[0].message.contains(HELD_BY_NOBODY),
            "{name}: a claim another file in the diff took over is held by somebody: {}",
            reported[0].message
        );
    }
}

/// The sentence T2 may only write when none of the claims that left arrived
/// anywhere in the change.
const HELD_BY_NOBODY: &str = "held by nobody";
