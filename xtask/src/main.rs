//! weed's own bench. The binary is the product; this is what the product is
//! measured with, and it never ships inside it.
//!
//! `cargo xtask mutate` is the recall campaign: real commits from the corpus,
//! one anti-pattern planted per case, judged by the release binary, written up
//! in `docs/calibration-2026-09.md` beside the precision the calibration loop
//! measures on the same commits.

mod mutate;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "the measurements weed makes about itself")]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Inject one anti-pattern per case into real commits and write down what
    /// weed caught.
    Mutate(mutate::Request),
}

/// The exit codes weed itself uses: 0 done, 1 the task failed, 3 it could not
/// run at all. A measurement that could not be made must never read like one
/// that came back clean.
fn main() -> ExitCode {
    let cli = Cli::parse();
    let answer = match &cli.task {
        Task::Mutate(request) => mutate::run(request),
    };
    match answer {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("{reason}");
            ExitCode::from(3)
        }
    }
}
