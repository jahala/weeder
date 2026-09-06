//! weed's own measurements, run from the workspace: `cargo xtask calibrate`
//! judges real history and writes the calibration file, `cargo xtask
//! suppressions` counts the allowances each repository wrote against its gate,
//! and `cargo xtask mutate` plants one anti-pattern per case in that same
//! history and writes the recall section of the same file.
//!
//! They are a workspace member rather than a script so they judge with the same
//! core the binary ships. There is one diff walk in this repository, and it is
//! the one in `src/`.

mod audit;
mod audit_packet;
mod calibrate;
mod corpus;
mod first_run;
mod judgement;
mod mutate;
mod repo;
mod report;
mod ruling;
mod split;
mod suppressions;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::judgement::Ledger;
use crate::report::Report;
use crate::split::Split;

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "weed's own measurements")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Judge the last commits of each corpus repository against their parents
    /// and write the calibration file.
    Calibrate(CalibrateArgs),
    /// Count the `Weed-allow:` trailers each corpus repository wrote per hundred
    /// commits, from the day it installed guard.
    Suppressions(SuppressionsArgs),
    /// Inject one anti-pattern per case into real commits of the corpus and
    /// write down what weed caught.
    Mutate(mutate::Request),
    /// Write a blind audit packet from pinned corpus diffs and rule ids.
    AuditPacket(audit_packet::Request),
}

#[derive(Debug, Args)]
struct CorpusArgs {
    /// Read the corpus from here instead of docs/calibration/corpus.toml.
    #[arg(long, value_name = "path")]
    corpus: Option<PathBuf>,
    /// Measure only these repositories, by name. Repeat it for more.
    #[arg(long, value_name = "name")]
    only: Vec<String>,
}

#[derive(Debug, Args)]
struct CalibrateArgs {
    #[command(flatten)]
    corpus: CorpusArgs,
    /// Judge at most this many commits per repository.
    #[arg(long, value_name = "n", default_value_t = calibrate::WINDOW)]
    limit: usize,
    /// Read the classifications from here instead of
    /// docs/calibration/judgements.toml.
    #[arg(long, value_name = "path")]
    judgements: Option<PathBuf>,
    /// Read the first run's block-level findings from here instead of
    /// docs/calibration/first-run.toml.
    #[arg(long, value_name = "path")]
    first_run: Option<PathBuf>,
    /// Read the independent re-grade from here instead of every
    /// docs/calibration-audit*.md the repository carries. Repeat it to name
    /// more than one.
    #[arg(long, value_name = "path")]
    audit: Vec<PathBuf>,
    /// Write the report here instead of docs/calibration-2026-09.md.
    #[arg(long, value_name = "path")]
    out: Option<PathBuf>,
    /// Also write every finding of every blocked commit here, as json. It is
    /// what whoever classifies a block reads.
    #[arg(long, value_name = "path")]
    findings: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct SuppressionsArgs {
    #[command(flatten)]
    corpus: CorpusArgs,
    /// Print a table or json.
    #[arg(long, value_enum, value_name = "format", default_value_t = RateFormat::Table)]
    format: RateFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum RateFormat {
    Table,
    Json,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Command::Calibrate(args) => run_calibrate(&args),
        Command::Suppressions(args) => run_suppressions(&args),
        Command::Mutate(request) => Ok(mutate::run(&request)?),
        Command::AuditPacket(request) => Ok(audit_packet::run(&request, &root())?),
    }
}

fn run_calibrate(args: &CalibrateArgs) -> Result<(), Box<dyn Error>> {
    let repos = corpus(&args.corpus)?;
    let ledger = Ledger::read(
        &args
            .judgements
            .clone()
            .unwrap_or_else(|| root().join("docs/calibration/judgements.toml")),
    )?;
    let first_path = args
        .first_run
        .clone()
        .unwrap_or_else(|| root().join(first_run::DEFAULT_PATH));
    let first = first_run::read(&first_path, &labelled(&first_path))?;
    // The files a caller points at, which is how the suites probe the wording;
    // otherwise every audit the repository carries.
    let audit = if args.audit.is_empty() {
        audit::read_all(&root())
    } else {
        let named: Vec<(PathBuf, String)> = args
            .audit
            .iter()
            .map(|path| (path.clone(), labelled(path)))
            .collect();
        audit::read_many(&named)
    };
    let scratch = tempfile::tempdir()?;

    let mut measurements = Vec::new();
    let mut rates = Vec::new();
    for repo in &repos {
        eprintln!("judging {} …", repo.name);
        measurements.push(calibrate::measure(
            repo,
            scratch.path(),
            args.limit,
            &first.commits_of(&repo.name),
        )?);
        rates.push(suppressions::measure(repo, scratch.path())?);
    }

    let split = Split::measure(&first, &measurements);
    // The ruling's record is printed into the repository's own report only:
    // a bench report written somewhere else describes a history that never
    // had a ruling.
    let ruling = if args.out.is_none() {
        ruling::read(&root().join(ruling::DEFAULT_PATH))?
    } else {
        None
    };
    let report = Report {
        ruling: ruling.as_ref(),
        repos: &measurements,
        rates: &rates,
        ledger: &ledger,
        split: &split,
        audit: &audit,
    };
    let outcome = report.outcome();
    let canonical = root().join("docs/calibration-2026-09.md");
    let out = args.out.clone().unwrap_or_else(|| canonical.clone());
    write(&out, &with_recall(&report.render(), &canonical))?;

    if let Some(path) = &args.findings {
        write(path, &findings_json(&measurements))?;
    }

    println!(
        "{} commits judged, {} blocked: {} true positive, {} acceptable, {} false positive, {} unclassified",
        outcome.judged,
        outcome.blocked,
        outcome.tally.true_positive,
        outcome.tally.acceptable,
        outcome.tally.false_positive,
        outcome.tally.unclassified,
    );
    println!("{} written", out.display());
    if outcome.tally.unclassified > 0 {
        println!(
            "{} blocked commits are unclassified and count as false positives until somebody reads them.",
            outcome.tally.unclassified
        );
    }
    Ok(())
}

fn run_suppressions(args: &SuppressionsArgs) -> Result<(), Box<dyn Error>> {
    let repos = corpus(&args.corpus)?;
    let scratch = tempfile::tempdir()?;
    let mut rates = Vec::new();
    for repo in &repos {
        rates.push(suppressions::measure(repo, scratch.path())?);
    }

    match args.format {
        RateFormat::Json => println!("{}", rates_json(&rates)),
        RateFormat::Table => {
            println!(
                "{:<10} {:>8} {:>10} {:>9} {:>9}  guard installed",
                "repo", "commits", "since", "trailers", "per 100"
            );
            for rate in &rates {
                println!(
                    "{:<10} {:>8} {:>10} {:>9} {:>9.1}  {}",
                    rate.name,
                    rate.commits,
                    rate.commits_since,
                    rate.trailers_since,
                    rate.per_hundred(),
                    match &rate.installed {
                        Some(installed) => format!("{} {}", installed.day, &installed.sha[..10]),
                        None => "never".to_string(),
                    }
                );
            }
        }
    }
    Ok(())
}

fn corpus(args: &CorpusArgs) -> Result<Vec<corpus::Repo>, Box<dyn Error>> {
    let path = args
        .corpus
        .clone()
        .unwrap_or_else(|| root().join(corpus::DEFAULT_PATH));
    Ok(corpus::select(corpus::read(&path)?, &args.only))
}

/// A path as the report names it: relative to the workspace root where it sits
/// under it, so the file the run wrote reads the same on any machine.
fn labelled(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// The workspace root, found from where this crate sits rather than from the
/// directory cargo was called in: `cargo xtask` writes the same file wherever it
/// is run from.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

/// The precision half with the recall half kept beneath it. `cargo xtask mutate`
/// writes its section between markers in the calibration file and this
/// measurement never reads a case of it; whatever section the file in the tree
/// carries is carried again, so the two measurements share one file without
/// either writing over the other.
fn with_recall(rendered: &str, canonical: &Path) -> String {
    let existing = std::fs::read_to_string(canonical).unwrap_or_default();
    match mutate::recall_section(&existing) {
        Some(section) => format!("{rendered}\n{section}\n"),
        None => rendered.to_string(),
    }
}

fn write(path: &Path, content: &str) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

/// Every finding of every blocked commit, as json, so a classification is
/// written from what weed actually said rather than from the rule id alone.
fn findings_json(measurements: &[calibrate::RepoMeasurement]) -> String {
    let repos: Vec<serde_json::Value> = measurements
        .iter()
        .map(|measurement| {
            serde_json::json!({
                "repo": measurement.name,
                "source": measurement.source,
                "tip": measurement.tip,
                "judged": measurement.judged,
                "warned": measurement.warned,
                "clean": measurement.clean,
                "blocked": measurement.blocked.iter().map(|blocked| serde_json::json!({
                    "sha": blocked.sha,
                    "parent": blocked.parent,
                    "subject": blocked.subject,
                    "rules": blocked.rules,
                    "findings": blocked.findings.iter().map(|finding| serde_json::json!({
                        "rule": finding.rule,
                        "level": finding.level,
                        "path": finding.path,
                        "line": finding.line,
                        "message": finding.message,
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::Value::Array(repos)).unwrap_or_default()
    )
}

fn rates_json(rates: &[suppressions::RepoRate]) -> String {
    let rates: Vec<serde_json::Value> = rates
        .iter()
        .map(|rate| {
            serde_json::json!({
                "repo": rate.name,
                "tip": rate.tip,
                "commits": rate.commits,
                "installed": rate.installed.as_ref().map(|installed| serde_json::json!({
                    "sha": installed.sha,
                    "day": installed.day,
                })),
                "commits_since_install": rate.commits_since,
                "trailers_since_install": rate.trailers_since,
                "trailers_before_install": rate.trailers_before,
                "per_hundred_commits": rate.per_hundred(),
            })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::Value::Array(rates)).unwrap_or_default()
}
