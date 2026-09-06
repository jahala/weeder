//! weed's own measurements, run from the workspace: `cargo xtask calibrate`
//! judges real history and writes the calibration file, `cargo xtask
//! suppressions` counts the allowances each repository wrote against its gate.
//!
//! They are a workspace member rather than a script so they judge with the same
//! core the binary ships. There is one diff walk in this repository, and it is
//! the one in `src/`.

mod calibrate;
mod corpus;
mod judgement;
mod repo;
mod report;
mod suppressions;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::judgement::Ledger;
use crate::report::Report;

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
}

#[derive(Debug, Args)]
struct CorpusArgs {
    /// Read the corpus from here instead of docs/calibration/corpus.toml.
    #[arg(long, value_name = "path")]
    corpus: Option<PathBuf>,
    /// Override or add one repository, as <name>=<path>. Repeat it for more.
    #[arg(long, value_name = "name=path")]
    repo: Vec<String>,
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
    let scratch = tempfile::tempdir()?;

    let mut measurements = Vec::new();
    let mut rates = Vec::new();
    for repo in &repos {
        eprintln!("judging {} …", repo.name);
        measurements.push(calibrate::measure(repo, scratch.path(), args.limit)?);
        rates.push(suppressions::measure(repo, scratch.path())?);
    }

    let report = Report {
        repos: &measurements,
        rates: &rates,
        ledger: &ledger,
    };
    let outcome = report.outcome();
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| root().join("docs/calibration-2026-09.md"));
    write(&out, &report.render())?;

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
    Ok(corpus::select(corpus::read(&path, &args.repo)?, &args.only))
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
                "reference": measurement.reference,
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
                "reference": rate.reference,
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
