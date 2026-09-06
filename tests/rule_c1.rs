//! C1, a guardrail was edited.
//!
//! A guardrail decides what the other checks do, so any change to one is the
//! finding: the harness settings, the hook, weeder's own law. The two markdown
//! files are read section by section instead, because most of what they hold is
//! prose and one section of them is law.
//!
//! The neighbours are the three that look the same and are not: a file under
//! `.github/` that owns nothing but review, a paragraph of `AGENTS.md` three
//! sections away from the limits, and a workflow, which is a by-law C3 warns
//! about rather than a constitution C1 stops a commit over.

mod common;

use common::{fixture, fixture_file, Repo};

/// The fixture's languages folder. C1 reads paths, so one repository proves it.
const CASE: &str = "paths";

/// Every guardrail path the rule names, as the fire fixture writes them.
const GUARDRAILS: [&str; 5] = [
    ".claude/settings.json",
    ".codex/config.toml",
    ".gemini/settings.json",
    ".githooks/pre-commit",
    "weeder.toml",
];

/// The by-law the fire fixture edits alongside them. It is a change C3 reports,
/// and C1 has nothing to say about it.
const WORKFLOW: &str = ".github/workflows/ci.yml";

/// The instructions file, whose law lives in one section.
const INSTRUCTIONS: &str = "AGENTS.md";

/// The heading that opens the section those files state their law in.
const LIMITS: &str = "hard limits";

#[test]
fn c1_fires_at_block_level_on_every_guardrail_path_and_inside_the_hard_limits() {
    let repo = fixture("C1", CASE, "fire");
    let run = repo.weeder(&["check"]);

    assert_eq!(run.code, 2, "a guardrail edit blocks\n{}", run.stderr);
    let findings: Vec<common::Finding> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == "C1")
        .collect();
    let mut expected: Vec<String> = GUARDRAILS.iter().map(ToString::to_string).collect();
    expected.push(INSTRUCTIONS.to_string());
    expected.sort();
    let mut named: Vec<String> = findings
        .iter()
        .map(|finding| finding.path.clone())
        .collect();
    named.sort();
    named.dedup();
    assert_eq!(
        named, expected,
        "every guardrail is named, and nothing else is"
    );
    for finding in &findings {
        assert_eq!(finding.level, "error", "C1 blocks");
    }

    let changed = repo.git(&["diff", "HEAD", "--name-only"]);
    assert!(
        changed.lines().any(|line| line == WORKFLOW),
        "the workflow must be in the diff, or the silence about it proves nothing"
    );
    assert!(
        run.findings()
            .iter()
            .any(|finding| finding.rule == "C3" && finding.path == WORKFLOW),
        "the workflow edit is C3's to report"
    );

    let after = fixture_file("C1", CASE, "fire/after", INSTRUCTIONS);
    let before = fixture_file("C1", CASE, "fire/before", INSTRUCTIONS);
    let section = limits_section(&after).expect("the fixture states hard limits");
    let reported = findings
        .iter()
        .find(|finding| finding.path == INSTRUCTIONS)
        .and_then(|finding| finding.line)
        .expect("the instructions finding names the line it landed on");
    assert!(
        section.contains(&reported),
        "the finding lands inside the hard-limits section, not somewhere in the prose"
    );
    let touched = after
        .lines()
        .nth(reported as usize - 1)
        .expect("the reported line is a line of the file");
    assert!(
        !before.lines().any(|line| line == touched),
        "the reported line has to be one the change wrote: {touched}"
    );
}

#[test]
fn c1_stays_silent_on_codeowners_and_on_a_hunk_elsewhere_in_the_instructions() {
    let repo = fixture("C1", CASE, "silent");

    let changed = repo.git(&["diff", "HEAD", "--name-only"]);
    for path in [".github/CODEOWNERS", INSTRUCTIONS] {
        assert!(
            changed.lines().any(|line| line == path),
            "{path} must be in the diff, or the silence proves nothing"
        );
    }
    let after = fixture_file("C1", CASE, "silent/after", INSTRUCTIONS);
    let before = fixture_file("C1", CASE, "silent/before", INSTRUCTIONS);
    let section = limits_section(&after).expect("the neighbour still states hard limits");
    for line in section {
        assert_eq!(
            after.lines().nth(line as usize - 1),
            before.lines().nth(line as usize - 1),
            "the neighbour leaves the limits exactly as they were"
        );
    }

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "a review file and a paragraph of prose are not guardrails"
    );
    assert_eq!(run.code, 0, "nothing found, nothing blocked");
}

/// The 1-based lines the hard-limits heading owns: itself, and everything under
/// it until a heading of its own rank or above.
fn limits_section(text: &str) -> Option<std::ops::RangeInclusive<u64>> {
    let lines: Vec<&str> = text.lines().collect();
    let (start, depth) = lines.iter().enumerate().find_map(|(index, line)| {
        let rank = rank(line)?;
        line.to_ascii_lowercase()
            .contains(LIMITS)
            .then_some((index, rank))
    })?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| rank(line).is_some_and(|next| next <= depth))
        .map_or(lines.len(), |(index, _)| index);
    Some(start as u64 + 1..=end as u64)
}

fn rank(line: &str) -> Option<usize> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    (hashes > 0 && line[hashes..].starts_with(' ')).then_some(hashes)
}

#[test]
fn c1_stays_silent_on_weeders_own_hooks_and_fires_once_one_is_edited() {
    let repo = Repo::init();
    let installed = repo.weeder(&["guard", "install", "--protect", "main"]);
    assert_eq!(
        installed.code, 0,
        "install runs clean\n{}",
        installed.stderr
    );
    repo.stage_all();

    let adoption = repo.weeder(&["check", "--staged", "--strict"]);
    assert_eq!(
        adoption.findings(),
        Vec::new(),
        "the bundle weeder wrote is not a guardrail edit: adopting weeder goes through the gate"
    );
    assert_eq!(adoption.code, 0);

    let hook = repo.root().join(".githooks/pre-commit");
    let mut text = std::fs::read_to_string(&hook).expect("the hook weeder wrote is readable");
    text = text.replace("set -eu\n", "set -eu\nexit 0\n");
    std::fs::write(&hook, text).expect("the hook is writable");
    repo.stage_all();

    let edited = repo.weeder(&["check", "--staged", "--strict"]);
    assert_eq!(
        edited.code, 2,
        "one line added to the bundle is a guardrail edit again"
    );
    assert_eq!(
        edited.paths(),
        vec![".githooks/pre-commit"],
        "only the rewritten hook is named; the two untouched bundles stay silent"
    );
    assert!(edited.findings().iter().all(|finding| finding.rule == "C1"));
}
