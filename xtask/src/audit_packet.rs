//! A blind packet for the calibration audit.
//!
//! The sighted audit samples the calibration report itself, where the builder's
//! class is printed beside each commit. This command draws a fresh sample, then
//! returns to the pinned corpus for the bytes a blind reader should see: case
//! identity, rule id, and the verbatim diff produced by git. It does not read
//! the judgement ledger.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use sha2::{Digest, Sha256};
use weed::faces::{check, rules, Format};

use crate::calibrate;
use crate::corpus;
use crate::mutate;
use crate::repo::{self, Scratch};

const SAMPLE_SIZE: usize = 20;
const DIFF_CAP_BYTES: usize = 75_000;
const DEFAULT_REPORT: &str = "docs/calibration-2026-09.md";
const DEFAULT_GO_CORPUS: &str = "docs/calibration/corpus-go.toml";

#[derive(Debug, Args, Clone)]
pub struct Request {
    /// Seed for the reproducible shuffle.
    #[arg(long)]
    pub seed: String,
    /// Read the calibration report from here.
    #[arg(long, value_name = "path")]
    pub report: Option<PathBuf>,
    /// Read calibration repositories from here.
    #[arg(long, value_name = "path")]
    pub corpus: Option<PathBuf>,
    /// Read recall's Go repositories from here.
    #[arg(long = "go-corpus", value_name = "path")]
    pub go_corpus: Option<PathBuf>,
    /// Write one packet file per sampled case into this directory.
    #[arg(long, value_name = "dir")]
    pub dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BlockedCase {
    repo: String,
    sha: String,
    rules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecallCase {
    rule: String,
    lang: String,
    repo: String,
    sha: String,
    path: String,
}

pub fn run(request: &Request, root: &Path) -> Result<(), Box<dyn Error>> {
    let report = std::fs::read_to_string(
        request
            .report
            .clone()
            .unwrap_or_else(|| root.join(DEFAULT_REPORT)),
    )?;
    if let Some(dir) = &request.dir {
        let written = write_case_packets(request, root, &report, dir)?;
        println!(
            "wrote {} blind audit case packets to {}",
            written,
            dir.display()
        );
    } else {
        print!("{}", packet(request, root, &report)?);
    }
    Ok(())
}

fn write_case_packets(
    request: &Request,
    root: &Path,
    report: &str,
    dir: &Path,
) -> Result<usize, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let material = material(request, root, report)?;
    let rules_table = rules::run(rules::Format::Table).stdout;
    let mut written = 0;

    for case in material.blocked {
        let scratch = material
            .fetched
            .get(&case.repo)
            .ok_or_else(|| format!("{} is not named by the corpus files", case.repo))?;
        let sha = full_sha(scratch.path(), &case.sha)?;
        let parent = parent(scratch.path(), &sha)?;
        scratch.checkout(&sha)?;
        let findings = blocked_findings(scratch.path(), &parent, &case.rules)?;
        let diff = diff(scratch.path(), &parent, &sha, &[])?;
        let body = case_packet(
            &request.seed,
            &format!("blocked:{}:{}", case.repo, case.sha),
            &format!("Rules: {}", case.rules.join(", ")),
            &catalogue_lines(&rules_table, &case.rules),
            &findings_table(&findings),
            None,
            &diff,
        );
        fs::write(
            dir.join(format!(
                "{}.md",
                case_id(&format!("blocked:{}:{}", case.repo, case.sha))
            )),
            masked(&body),
        )?;
        written += 1;
    }

    for case in material.recall {
        let repo = material
            .repo_defs
            .get(&case.repo)
            .ok_or_else(|| format!("{} is not named by the corpus files", case.repo))?;
        let replay = mutate::replay_case(repo, &case.sha, &case.rule, &case.lang)?;
        let extra = format!(
            "Planted site: {}\n\nPlanted shape: {}\n\nQuestion: is rule {}'s shape genuinely present at that planted site, and did weed report it there?",
            planted_site(&replay.target, replay.lines),
            replay.shape,
            case.rule
        );
        let body = case_packet(
            &request.seed,
            &format!(
                "recall:{}:{}:{}:{}:{}",
                case.rule, case.lang, case.repo, case.sha, case.path
            ),
            &format!("Rule: {}", case.rule),
            &catalogue_lines(&rules_table, std::slice::from_ref(&case.rule)),
            &findings_table(&replay.findings),
            Some(&extra),
            &replay.diff,
        );
        fs::write(
            dir.join(format!(
                "{}.md",
                case_id(&format!(
                    "recall:{}:{}:{}:{}:{}",
                    case.rule, case.lang, case.repo, case.sha, case.path
                ))
            )),
            masked(&body),
        )?;
        written += 1;
    }

    Ok(written)
}

struct PacketMaterial {
    blocked: Vec<BlockedCase>,
    recall: Vec<RecallCase>,
    repo_defs: BTreeMap<String, corpus::Repo>,
    fetched: BTreeMap<String, Scratch>,
    _scratch: tempfile::TempDir,
}

fn material(
    request: &Request,
    root: &Path,
    report: &str,
) -> Result<PacketMaterial, Box<dyn Error>> {
    let blocked = seeded_sample(parse_blocked(report), &request.seed, "blocked");
    let recall = seeded_sample(parse_recall(report), &request.seed, "recall");
    if blocked.len() < SAMPLE_SIZE || recall.len() < SAMPLE_SIZE {
        return Err(format!(
            "the report yielded {} blocked cases and {} recall cases; each blind audit sample needs {SAMPLE_SIZE}",
            blocked.len(),
            recall.len()
        )
        .into());
    }

    let mut repos = corpus::read(
        &request
            .corpus
            .clone()
            .unwrap_or_else(|| root.join(corpus::DEFAULT_PATH)),
    )?;
    repos.extend(corpus::read(
        &request
            .go_corpus
            .clone()
            .unwrap_or_else(|| root.join(DEFAULT_GO_CORPUS)),
    )?);

    let scratch = tempfile::tempdir()?;
    let mut repo_defs: BTreeMap<String, corpus::Repo> = BTreeMap::new();
    let mut fetched: BTreeMap<String, Scratch> = BTreeMap::new();
    for repo in repos {
        if fetched.contains_key(&repo.name) {
            continue;
        }
        repo_defs.insert(repo.name.clone(), repo.clone());
        fetched.insert(
            repo.name.clone(),
            Scratch::fetch(scratch.path(), &repo.name, &repo.source, &repo.tip)?,
        );
    }

    Ok(PacketMaterial {
        blocked,
        recall,
        repo_defs,
        fetched,
        _scratch: scratch,
    })
}

fn packet(request: &Request, root: &Path, report: &str) -> Result<String, Box<dyn Error>> {
    let blocked = seeded_sample(parse_blocked(report), &request.seed, "blocked");
    let recall = seeded_sample(parse_recall(report), &request.seed, "recall");
    if blocked.len() < SAMPLE_SIZE || recall.len() < SAMPLE_SIZE {
        return Err(format!(
            "the report yielded {} blocked cases and {} recall cases; each blind audit sample needs {SAMPLE_SIZE}",
            blocked.len(),
            recall.len()
        )
        .into());
    }

    let mut repos = corpus::read(
        &request
            .corpus
            .clone()
            .unwrap_or_else(|| root.join(corpus::DEFAULT_PATH)),
    )?;
    repos.extend(corpus::read(
        &request
            .go_corpus
            .clone()
            .unwrap_or_else(|| root.join(DEFAULT_GO_CORPUS)),
    )?);

    let scratch = tempfile::tempdir()?;
    let mut repo_defs: BTreeMap<String, corpus::Repo> = BTreeMap::new();
    let mut fetched: BTreeMap<String, Scratch> = BTreeMap::new();
    for repo in repos {
        if fetched.contains_key(&repo.name) {
            continue;
        }
        repo_defs.insert(repo.name.clone(), repo.clone());
        fetched.insert(
            repo.name.clone(),
            Scratch::fetch(scratch.path(), &repo.name, &repo.source, &repo.tip)?,
        );
    }

    let mut out = String::new();
    out.push_str("# calibration audit blind packet, 2026-09\n\n");
    out.push_str("This packet was generated by `cargo xtask audit-packet` from the pinned corpus. It contains case identifiers, rule catalogue text, weed findings, verbatim `git diff` output and recall planting questions only. It does not contain judgement rows or audit verdicts.\n\n");
    out.push_str(&format!("Seed: {}\n", request.seed));
    out.push_str("Samples: 20 blocked commits, 20 recall cases\n\n");
    out.push_str("Reply with two markdown tables named `Blocked Commit Sample` and `Recall Case Sample`. Use verdicts `true-positive`, `acceptable`, `false-positive` for blocked commits and `miss` or `caught` for recall cases. Give one short reason for each row.\n\n");

    let rules_table = rules::run(rules::Format::Table).stdout;
    out.push_str("## Blocked Commit Packet\n\n");
    for case in blocked {
        let scratch = fetched
            .get(&case.repo)
            .ok_or_else(|| format!("{} is not named by the corpus files", case.repo))?;
        let sha = full_sha(scratch.path(), &case.sha)?;
        let parent = parent(scratch.path(), &sha)?;
        scratch.checkout(&sha)?;
        let findings = blocked_findings(scratch.path(), &parent, &case.rules)?;
        let paths = finding_paths(&findings);
        out.push_str(&format!(
            "### blocked:{}:{}\n\nRules: {}\n\n{}\n{}\n```diff\n{}\n```\n\n",
            case.repo,
            case.sha,
            case.rules.join(", "),
            catalogue_lines(&rules_table, &case.rules),
            findings_table(&findings),
            diff(scratch.path(), &parent, &sha, &paths)?
        ));
    }

    out.push_str("## Recall Case Packet\n\n");
    for case in recall {
        let repo = repo_defs
            .get(&case.repo)
            .ok_or_else(|| format!("{} is not named by the corpus files", case.repo))?;
        let replay = mutate::replay_case(repo, &case.sha, &case.rule, &case.lang)?;
        out.push_str(&format!(
            "### recall:{}:{}:{}:{}:{}\n\nRule: {}\n\n{}\n{}\nPlanted site: {}\n\nPlanted shape: {}\n\nQuestion: is rule {}'s shape genuinely present at that planted site, and did weed report it there?\n\n```diff\n{}\n```\n\n",
            case.rule,
            case.lang,
            case.repo,
            case.sha,
            case.path,
            case.rule,
            catalogue_lines(&rules_table, std::slice::from_ref(&case.rule)),
            findings_table(&replay.findings),
            planted_site(&replay.target, replay.lines),
            replay.shape,
            case.rule,
            replay.diff
        ));
    }

    Ok(out)
}

fn full_sha(repo: &Path, short: &str) -> Result<String, Box<dyn Error>> {
    Ok(repo::git(
        repo,
        &["rev-parse", "--verify", &format!("{short}^{{commit}}")],
    )?)
}

fn parent(repo: &Path, sha: &str) -> Result<String, Box<dyn Error>> {
    Ok(repo::git(
        repo,
        &["rev-parse", "--verify", &format!("{sha}^")],
    )?)
}

fn diff(repo: &Path, parent: &str, sha: &str, paths: &[String]) -> Result<String, Box<dyn Error>> {
    if paths.is_empty() {
        return Ok(repo::git(repo, &["diff", "--no-ext-diff", parent, sha])?);
    }
    let mut arguments = vec![
        "diff".to_string(),
        "--no-ext-diff".to_string(),
        parent.to_string(),
        sha.to_string(),
        "--".to_string(),
    ];
    arguments.extend(paths.iter().cloned());
    let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
    Ok(repo::git(repo, &borrowed)?)
}

fn blocked_findings(
    repo: &Path,
    parent: &str,
    rules: &[String],
) -> Result<Vec<calibrate::Finding>, Box<dyn Error>> {
    let answer = check::run(&check::Request {
        cwd: repo.to_path_buf(),
        base: Some(parent.to_string()),
        tip: None,
        staged: false,
        scope: Vec::new(),
        strict: true,
        format: Format::Sarif,
        config: None,
        message_file: None,
        version: env!("CARGO_PKG_VERSION").to_string(),
    });
    let mut findings: Vec<calibrate::Finding> = calibrate::results(&answer.stdout)?
        .into_iter()
        .filter(|finding| {
            finding.level == "error" && rules.iter().any(|rule| rule == &finding.rule)
        })
        .collect();
    findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.rule.cmp(&right.rule))
            .then(left.message.cmp(&right.message))
    });
    Ok(findings)
}

fn finding_paths(findings: &[calibrate::Finding]) -> Vec<String> {
    let mut paths: Vec<String> = findings
        .iter()
        .map(|finding| finding.path.clone())
        .filter(|path| !path.is_empty())
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

fn catalogue_lines(table: &str, rules: &[String]) -> String {
    let mut out = String::from("Catalogue:\n\n```text\n");
    for line in table.lines() {
        if rules
            .iter()
            .any(|rule| line.split_whitespace().next() == Some(rule.as_str()))
        {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("```\n\n");
    out
}

fn findings_table(findings: &[calibrate::Finding]) -> String {
    let mut out = String::from(
        "Weed findings:\n\n| Rule | Level | Path | Line | Message |\n|---|---|---|---:|---|\n",
    );
    if findings.is_empty() {
        out.push_str("| none | none |  | 0 | weed printed no finding for this replayed site |\n\n");
        return out;
    }
    for finding in findings {
        out.push_str(&format!(
            "| {} | {} | `{}` | {} | {} |\n",
            finding.rule,
            finding.level,
            finding.path,
            finding.line,
            finding.message.replace('|', "\\|")
        ));
    }
    out.push('\n');
    out
}

fn case_packet(
    seed: &str,
    case: &str,
    summary: &str,
    catalogue: &str,
    findings: &str,
    extra: Option<&str>,
    diff: &str,
) -> String {
    let mut out = String::new();
    out.push_str("# calibration audit blind case packet, 2026-09\n\n");
    out.push_str("Generated by `cargo xtask audit-packet --dir` from the pinned corpus. This file is the auditor session's only input for this case. It contains machine-derived case identity, rule catalogue text, weed findings and verbatim git diff bytes when the diff fits the stated cap.\n\n");
    out.push_str(&format!("Seed: {seed}\n"));
    out.push_str(&format!("Case: {case}\n"));
    out.push_str(&format!("Diff cap bytes: {DIFF_CAP_BYTES}\n"));
    out.push_str(&format!("Diff bytes: {}\n", diff.len()));
    out.push_str(&format!(
        "Diff capped: {}\n\n",
        if diff.len() > DIFF_CAP_BYTES {
            "yes"
        } else {
            "no"
        }
    ));
    out.push_str(summary);
    out.push_str("\n\n");
    out.push_str(catalogue);
    out.push_str(findings);
    if let Some(extra) = extra {
        out.push_str(extra);
        out.push_str("\n\n");
    }
    if diff.len() > DIFF_CAP_BYTES {
        out.push_str("```text\n");
        out.push_str("The full git diff for this case exceeds the stated cap, so the generator omitted it instead of cutting it silently.\n");
        out.push_str("```\n");
    } else {
        out.push_str("```diff\n");
        out.push_str(diff);
        if !diff.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("```\n");
    }
    out
}

fn case_id(case: &str) -> String {
    case.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn planted_site(target: &[String], lines: Option<(u32, u32)>) -> String {
    let site = target.join(", ");
    match lines {
        Some((first, last)) if first == last => format!("{site}:{first}"),
        Some((first, last)) => format!("{site}:{first}-{last}"),
        None => site,
    }
}

fn seeded_sample<T: CaseKey + Clone>(items: Vec<T>, seed: &str, label: &str) -> Vec<T> {
    let mut scored: Vec<(String, T)> = items
        .into_iter()
        .map(|item| {
            let mut hasher = Sha256::new();
            hasher.update(seed.as_bytes());
            hasher.update([0]);
            hasher.update(label.as_bytes());
            hasher.update([0]);
            hasher.update(item.key().as_bytes());
            (format!("{:x}", hasher.finalize()), item)
        })
        .collect();
    scored.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.key().cmp(&right.1.key())));
    scored
        .into_iter()
        .take(SAMPLE_SIZE)
        .map(|(_, item)| item)
        .collect()
}

trait CaseKey {
    fn key(&self) -> String;
}

impl CaseKey for BlockedCase {
    fn key(&self) -> String {
        format!("{}:{}", self.repo, self.sha)
    }
}

impl CaseKey for RecallCase {
    fn key(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.rule, self.lang, self.repo, self.sha, self.path
        )
    }
}

fn parse_blocked(report: &str) -> Vec<BlockedCase> {
    let mut cases = Vec::new();
    let mut repo = None;
    let mut before_recall = true;
    for line in report.lines() {
        if line.trim() == "<!-- recall:begin -->" {
            before_recall = false;
        }
        if !before_recall {
            continue;
        }
        if let Some(name) = repo_heading(line) {
            repo = Some(name);
            continue;
        }
        let Some(repo) = &repo else {
            continue;
        };
        let Some(cells) = table_cells(line) else {
            continue;
        };
        if cells.len() < 3 || cells[0].eq_ignore_ascii_case("commit") {
            continue;
        }
        let Some((sha, _subject)) = commit_cell(&cells[0]) else {
            continue;
        };
        if !classification(&cells[2]) {
            continue;
        }
        let rules = cells[1]
            .split(',')
            .map(|rule| rule.trim().to_string())
            .filter(|rule| !rule.is_empty())
            .collect();
        cases.push(BlockedCase {
            repo: repo.clone(),
            sha,
            rules,
        });
    }
    cases
}

fn parse_recall(report: &str) -> Vec<RecallCase> {
    let mut cases = Vec::new();
    let mut in_misses = false;
    for line in report.lines() {
        if line.trim() == "### Every miss" {
            in_misses = true;
            continue;
        }
        if in_misses && line.starts_with("### ") {
            break;
        }
        if !in_misses {
            continue;
        }
        let Some(rest) = line.trim().strip_prefix("- ") else {
            continue;
        };
        let parts: Vec<&str> = rest.split(" · ").collect();
        if parts.len() < 4 {
            continue;
        }
        let rule = parts[0].trim().to_string();
        let lang = parts[1].trim().to_string();
        let Some((repo, sha)) = repo_and_sha(parts[2]) else {
            continue;
        };
        let Some((path, _after_path)) = take_backtick(parts[3]) else {
            continue;
        };
        cases.push(RecallCase {
            rule,
            lang,
            repo,
            sha,
            path,
        });
    }
    cases
}

fn repo_heading(line: &str) -> Option<String> {
    let line = line.strip_prefix("## ")?;
    let (name, rest) = line.split_once(',')?;
    rest.contains(" commits judged")
        .then(|| name.trim().to_string())
}

fn table_cells(line: &str) -> Option<Vec<String>> {
    let line = line.trim();
    if !line.starts_with('|') {
        return None;
    }
    let cells: Vec<String> = line
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect();
    if cells
        .iter()
        .all(|cell| cell.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '))
    {
        return None;
    }
    Some(cells)
}

fn commit_cell(cell: &str) -> Option<(String, String)> {
    if let Some((sha, rest)) = take_backtick(cell) {
        return is_sha(&sha).then(|| (sha, rest.trim().to_string()));
    }
    let (sha, subject) = cell.split_once(' ')?;
    is_sha(sha).then(|| (sha.to_string(), subject.to_string()))
}

fn repo_and_sha(text: &str) -> Option<(String, String)> {
    let (repo, rest) = text.trim().split_once(' ')?;
    let (sha, _after) = take_backtick(rest)?;
    Some((repo.to_string(), sha))
}

fn classification(cell: &str) -> bool {
    matches!(
        cell.to_ascii_lowercase().as_str(),
        "true positive" | "true-positive" | "acceptable" | "false positive" | "false-positive"
    )
}

fn take_backtick(text: &str) -> Option<(String, &str)> {
    let start = text.find('`')?;
    let rest = &text[start + 1..];
    let end = rest.find('`')?;
    Some((rest[..end].to_string(), &rest[end + 1..]))
}

fn is_sha(value: &str) -> bool {
    (9..=40).contains(&value.len()) && value.chars().all(|character| character.is_ascii_hexdigit())
}

/// A published example credential in a hunk, written as a marker that names it.
/// The corpus quotes what vendors print in their manuals, X1 reports those at
/// note level, and a packet that carried one whole would be the one file in
/// this repository a scanner stops on. The marker is machine-derived and says
/// nothing an auditor could not read off the stamp.
fn masked(text: &str) -> String {
    weed::core::rules::check::x1::published_examples()
        .iter()
        .fold(text.to_string(), |text, (whole, stamp)| {
            text.replace(
                whole,
                &format!("<published example credential, stamp {stamp}>"),
            )
        })
}
