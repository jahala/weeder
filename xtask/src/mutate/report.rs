//! The recall section of `docs/calibration-2026-09.md`, and the misses kept on
//! disk beside it.
//!
//! The file is written between two markers, so the section belongs to this
//! command and the rest of the file belongs to whoever else writes there. Every
//! number in it is counted from the cases; nothing is carried over from a
//! previous run, and a rule with no cases says so rather than showing a
//! percentage of nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::inject::Mutation;
use super::source::Language;
use super::tree::Passed;
use super::{Case, Outcome, Plan};

const BEGIN: &str = "<!-- recall:begin -->";
const END: &str = "<!-- recall:end -->";

/// The level each rule reports at, asked of the binary rather than assumed.
pub fn levels(binary: &Path) -> Result<BTreeMap<String, String>, String> {
    let output = std::process::Command::new(binary)
        .args(["rules", "--format", "json"])
        .output()
        .map_err(|error| format!("{} rules: {error}", binary.display()))?;
    if !output.status.success() {
        return Err("weed rules refused to print the catalogue".to_string());
    }
    let catalogue: Vec<serde_json::Value> =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
            .map_err(|error| format!("weed rules wrote json weed could not read: {error}"))?;
    Ok(catalogue
        .into_iter()
        .filter_map(|rule| {
            let id = rule.get("id")?.as_str()?.to_string();
            let level = rule.get("level")?.as_str()?.to_string();
            Some((id, level))
        })
        .collect())
}

/// One rule in one language, counted.
struct Row {
    rule: String,
    level: String,
    lang: Language,
    cases: usize,
    hits: usize,
}

impl Row {
    fn recall(&self) -> Option<f64> {
        (self.cases > 0).then(|| self.hits as f64 * 100.0 / self.cases as f64)
    }
}

/// The recall section a file already carries, markers included, or `None` for
/// a file that has never had one. The precision half of the file is written by
/// another measurement, and this is how it keeps this half where it found it.
#[must_use]
pub fn section(existing: &str) -> Option<&str> {
    let first = existing.find(BEGIN)?;
    let last = existing.find(END)?;
    (last > first).then(|| &existing[first..last + END.len()])
}

/// Write the recall section into the file, leaving everything else in it alone.
pub fn write(
    out: &Path,
    outcomes: &[Outcome],
    levels: &BTreeMap<String, String>,
    plan: &Plan,
) -> Result<String, String> {
    let cases: Vec<&Case> = outcomes.iter().flat_map(|one| one.cases.iter()).collect();
    let mut rows = Vec::new();
    for rule in &plan.rules {
        for lang in &plan.languages {
            let counted: Vec<&&Case> = cases
                .iter()
                .filter(|case| case.rule == *rule && case.lang == *lang)
                .collect();
            rows.push(Row {
                rule: rule.clone(),
                level: levels
                    .get(rule)
                    .cloned()
                    .unwrap_or_else(|| "warn".to_string()),
                lang: *lang,
                cases: counted.len(),
                hits: counted.iter().filter(|case| case.caught).count(),
            });
        }
    }

    let mut section = String::new();
    section.push_str(BEGIN);
    section.push_str("\n\n## Recall\n\n");
    section.push_str(&opening(&rows, &cases));
    section.push_str("\n\n### The corpus\n\n");
    section.push_str(&corpus_table(outcomes));
    section.push_str("\n### Recall, per rule and language\n\n");
    section.push_str(&recall_table(&rows));
    section.push_str("\n### Every miss\n\n");
    section.push_str(&misses(&cases));
    section.push_str("\n### Where a shape had nowhere to go\n\n");
    section.push_str(&absences(outcomes, plan));
    section.push('\n');
    section.push_str(END);
    section.push('\n');

    let existing = std::fs::read_to_string(out).unwrap_or_default();
    let written = splice(&existing, &section);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    std::fs::write(out, &written).map_err(|error| format!("{}: {error}", out.display()))?;

    let hits = cases.iter().filter(|case| case.caught).count();
    Ok(format!(
        "{} cases, {hits} caught, {} missed, written to {}",
        cases.len(),
        cases.len() - hits,
        out.display()
    ))
}

/// The sentence the section opens with: what was measured, and whether the
/// block-level rules cleared the bar in every language.
fn opening(rows: &[Row], cases: &[&Case]) -> String {
    const BAR: f64 = 95.0;
    let blocking: Vec<&Row> = rows
        .iter()
        .filter(|row| BARRED.contains(&row.rule.as_str()))
        .collect();
    let short: Vec<String> = blocking
        .iter()
        .filter(|row| row.recall().is_none_or(|recall| recall < BAR))
        .map(|row| format!("{} in {}", row.rule, row.lang.slug()))
        .collect();
    let verdict = if short.is_empty() {
        format!(
            "Every rule that blocks, {}, catches at least {BAR:.0} percent of the anti-patterns \
             planted for it, in each of ts, py, rs and go.",
            BARRED.join(", ")
        )
    } else {
        format!(
            "The bar is not met: {} {} under {BAR:.0} percent.",
            short.join(", "),
            if short.len() == 1 { "is" } else { "are" }
        )
    };
    let hits = cases.iter().filter(|case| case.caught).count();
    format!(
        "{verdict} {} cases were planted one anti-pattern at a time in real commits, and {hits} of \
         them were caught on the site they were planted in.\n\nThe campaign is `cargo xtask \
         mutate`. For each case it checks out a real commit of a corpus repository, plants one \
         anti-pattern in its tree with a scanner that knows nothing about weed's detectors, and \
         runs `weed check --base <parent> --strict`. A case counts as caught only where the rule \
         fires on the file the shape was planted in, on the lines it was planted on where it has \
         lines. A site the unmutated commit already fires that rule on is passed over, so no hit \
         is inherited from the commit itself.",
        cases.len()
    )
}

/// The rules the loop holds to 95 percent.
const BARRED: &[&str] = &["T1", "T3", "S1", "X1", "C1", "G1"];

fn corpus_table(outcomes: &[Outcome]) -> String {
    let mut table = String::from(
        "| Repository | Source | Window ends at | Commits walked | Cases |\n|---|---|---|---|---|\n",
    );
    for outcome in outcomes {
        table.push_str(&format!(
            "| {} | `{}` | `{}` | {} | {} |\n",
            outcome.repo,
            outcome.source,
            outcome.tip,
            outcome.walked,
            outcome.cases.len()
        ));
    }
    table.push_str(
        "\nThe garden five are the repositories calibration measures precision on, read from \
         `docs/calibration/corpus.toml` at the commits it pins them at. They carry no Go between \
         them and weed judges Go, so the Go column is measured on two Go projects that \
         `docs/calibration/corpus-go.toml` pins the same way, read exactly as the five are.\n\n\
         Every window ends at the pin. A commit pushed to one of these sources later is outside \
         the walk, so it plants no case and moves no number, and two runs over one corpus write \
         this section byte for byte the same.\n",
    );
    let mut over: BTreeMap<Passed, usize> = BTreeMap::new();
    for outcome in outcomes {
        for (reason, counted) in &outcome.no_site {
            *over.entry(*reason).or_default() += counted;
        }
    }
    if !over.is_empty() {
        table
            .push_str("\nFiles the injector planted nothing in, counted once for each commit they were read at:\n\n");
        for (reason, counted) in &over {
            table.push_str(&format!("- {}: {counted}\n", reason.spelled()));
        }
    }
    table
}

fn recall_table(rows: &[Row]) -> String {
    let mut table =
        String::from("| Rule | Level | Language | Cases | Hits | Misses | Recall |\n|---|---|---|---|---|---|---|\n");
    for row in rows {
        let recall = match row.recall() {
            Some(recall) => format!("{recall:.1}%"),
            None => "no cases".to_string(),
        };
        table.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            row.rule,
            row.level,
            row.lang.slug(),
            row.cases,
            row.hits,
            row.cases - row.hits,
            recall
        ));
    }
    table
}

fn misses(cases: &[&Case]) -> String {
    let missed: Vec<&&Case> = cases.iter().filter(|case| !case.caught).collect();
    if missed.is_empty() {
        return "No case was missed.\n".to_string();
    }
    let mut written = String::from(
        "Each of these was planted and not reported. The before and after of every one is kept \
         under the campaign's cache directory, as `cases/<rule>/<language>/<repository>-<commit>`, \
         so a number nobody believes can be replayed by hand. `WEED_RECALL_CACHE` says where that \
         directory is; it sits under the temporary directory otherwise.\n\n",
    );
    for case in missed {
        let place = match case.line {
            Some(line) => format!("{}:{line}", case.path),
            None => case.path.clone(),
        };
        written.push_str(&format!(
            "- {} · {} · {} `{}` · `{place}`, {}\n",
            case.rule,
            case.lang.slug(),
            case.repo,
            case.sha.chars().take(9).collect::<String>(),
            case.shape
        ));
    }
    written
}

fn absences(outcomes: &[Outcome], plan: &Plan) -> String {
    let mut written = String::new();
    for rule in &plan.rules {
        for lang in &plan.languages {
            let key = (rule.clone(), *lang);
            let cases: usize = outcomes
                .iter()
                .flat_map(|outcome| outcome.cases.iter())
                .filter(|case| case.rule == *rule && case.lang == *lang)
                .count();
            if cases > 0 {
                continue;
            }
            let impossible = outcomes
                .iter()
                .any(|outcome| outcome.impossible.contains(&key));
            let looked: usize = outcomes
                .iter()
                .filter_map(|outcome| outcome.absent.get(&key))
                .sum();
            let passed: usize = outcomes
                .iter()
                .filter_map(|outcome| outcome.passed_over.get(&key))
                .sum();
            let reason = if impossible {
                "the language's runner does not collect by name, so no rename takes a case out of \
                 the run"
                    .to_string()
            } else {
                format!(
                    "no commit of the corpus held a site for this shape: {looked} commits offered \
                     none, and {passed} more were passed over because the commit itself already \
                     fires the rule there"
                )
            };
            written.push_str(&format!("- {rule} · {}, {reason}\n", lang.slug()));
        }
    }
    if written.is_empty() {
        "Every rule found a site in every language.\n".to_string()
    } else {
        written
    }
}

/// The file with the section put where the markers say, or added at the end of
/// a file that has never carried one.
fn splice(existing: &str, section: &str) -> String {
    if let (Some(first), Some(last)) = (existing.find(BEGIN), existing.find(END)) {
        if last > first {
            let mut written = String::new();
            written.push_str(&existing[..first]);
            written.push_str(section);
            written.push_str(existing[last + END.len()..].trim_start_matches('\n'));
            return written;
        }
    }
    let mut written = if existing.trim().is_empty() {
        preamble()
    } else {
        let mut carried = existing.to_string();
        if !carried.ends_with('\n') {
            carried.push('\n');
        }
        carried.push('\n');
        carried
    };
    written.push_str(section);
    written
}

/// What the file says before anyone has written the precision half of it. The
/// two measurements belong in one file, and this one refuses to imply the other
/// has been made.
fn preamble() -> String {
    String::from(
        "# Calibration, 2026-09\n\nweed is measured twice on the same code before it is allowed \
         to block anyone: precision, how often it stops a commit that was fine, and recall, how \
         often it catches an anti-pattern that is really there. A gate that fires on nothing has \
         perfect precision, which is why neither number means anything without the other.\n\n\
         The precision half of this file is written by `cargo xtask calibrate` and is not in it \
         yet. The recall half below is written by `cargo xtask mutate`.\n\n",
    )
}

/// Keep a missed case as a pair the fixture harness can be pointed at: the tree
/// as the commit left it, and the tree with the anti-pattern in it.
pub fn keep(
    cases_dir: &Path,
    repo: &str,
    sha: &str,
    rule: &str,
    lang: Language,
    planted: &Mutation,
    before: &[(String, Option<String>)],
) -> Result<(), String> {
    let short: String = sha.chars().take(9).collect();
    let case = cases_dir
        .join(rule)
        .join(lang.slug())
        .join(format!("{repo}-{short}"));
    write_side(&case.join("before"), before)?;
    let after: Vec<(String, Option<String>)> = planted.writes.clone();
    write_side(&case.join("after"), &after)?;
    let note = serde_json::json!({
        "rule": rule,
        "language": lang.slug(),
        "repository": repo,
        "commit": sha,
        "shape": planted.shape,
        "target": planted.target,
        "lines": planted.lines.map(|(first, last)| [first, last]),
        "arguments": planted.arguments,
        "config": planted.config,
    });
    std::fs::write(
        case.join("case.json"),
        serde_json::to_string_pretty(&note).unwrap_or_default(),
    )
    .map_err(|error| format!("{}: {error}", case.display()))
}

fn write_side(root: &Path, files: &[(String, Option<String>)]) -> Result<(), String> {
    std::fs::create_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))?;
    for (path, content) in files {
        let Some(text) = content else {
            continue;
        };
        let whole = root.join(path);
        if let Some(parent) = whole.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        std::fs::write(&whole, text).map_err(|error| format!("{}: {error}", whole.display()))?;
    }
    Ok(())
}

/// The whole run as json, for anything that would rather read numbers.
pub fn write_json(path: &PathBuf, outcomes: &[Outcome]) -> Result<(), String> {
    let cases: Vec<serde_json::Value> = outcomes
        .iter()
        .flat_map(|outcome| outcome.cases.iter())
        .map(|case| {
            serde_json::json!({
                "rule": case.rule,
                "language": case.lang.slug(),
                "repository": case.repo,
                "commit": case.sha,
                "path": case.path,
                "line": case.line,
                "shape": case.shape,
                "caught": case.caught,
            })
        })
        .collect();
    let repositories: Vec<serde_json::Value> = outcomes
        .iter()
        .map(|outcome| {
            serde_json::json!({
                "repository": outcome.repo,
                "source": outcome.source,
                "head": outcome.tip,
                "commits": outcome.walked,
            })
        })
        .collect();
    let document = serde_json::json!({ "repositories": repositories, "cases": cases });
    std::fs::write(
        path,
        serde_json::to_string_pretty(&document).unwrap_or_default(),
    )
    .map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_replaces_the_one_that_was_there() {
        let existing = format!("# one\n\ntext\n\n{BEGIN}\nold\n{END}\n\nafter\n");
        let written = splice(&existing, &format!("{BEGIN}\nnew\n{END}\n"));
        assert!(written.contains("new"));
        assert!(!written.contains("old"));
        assert!(written.contains("after"));
    }

    #[test]
    fn a_file_that_never_had_one_gets_a_preamble() {
        let written = splice("", &format!("{BEGIN}\nnew\n{END}\n"));
        assert!(written.starts_with("# Calibration"));
        assert!(written.contains("new"));
    }

    fn case(rule: &str, caught: bool) -> Case {
        Case {
            rule: rule.to_string(),
            lang: Language::Ts,
            repo: "pleach".to_string(),
            sha: "0e1dac6b10287f3f1ff516e8d4d568b9f049137b".to_string(),
            path: "test/unit/umbel-seam.test.ts".to_string(),
            line: Some(46),
            shape: "`test.skip` was put on a case".to_string(),
            caught,
        }
    }

    /// The one thing the recall number may not do is round a miss away. A case
    /// weed did not catch is named where anyone can go and replay it, and a run
    /// that missed something never reads like one that missed nothing.
    #[test]
    fn every_miss_is_named_by_repository_commit_and_site() {
        let missed = case("T3", false);
        let caught = case("T1", true);
        let written = misses(&[&caught, &missed]);
        assert!(
            written.contains("- T3 · ts · pleach `0e1dac6b1` · `test/unit/umbel-seam.test.ts:46`"),
            "the miss is not named: {written}"
        );
        assert!(
            written.contains("`test.skip` was put on a case"),
            "the miss does not say what was planted: {written}"
        );
        assert!(
            !written.contains("T1"),
            "a case weed caught is not a miss: {written}"
        );
        assert!(
            !written.contains("No case was missed"),
            "a run with a miss in it read as one with none: {written}"
        );
    }

    #[test]
    fn a_run_that_missed_nothing_says_so() {
        assert_eq!(misses(&[&case("T3", true)]), "No case was missed.\n");
    }
}
