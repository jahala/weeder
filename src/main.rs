//! The command line. Every subcommand parses its flags here, asks its face for
//! an `Answer`, writes it, and leaves with the code the contract names: 0 clean
//! or warnings only, 2 at least one block-level result, 3 weed could not run.

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use weed::core::sarif::EXIT_COULD_NOT_RUN;
use weed::faces::{check, rules, Answer};

#[derive(Debug, Parser)]
#[command(name = "weed", version, about = "the judge of the diff")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Judge a diff: the index and the working tree against HEAD by default.
    Check(CheckArgs),
    /// Print the rule catalogue and the level each rule carries.
    Rules(RulesArgs),
}

#[derive(Debug, Args)]
struct CheckArgs {
    /// Judge the tree against this ref instead of HEAD.
    #[arg(long, value_name = "ref")]
    base: Option<String>,
    /// Judge the index alone, the view a pre-commit hook has.
    #[arg(long)]
    staged: bool,
    /// The paths the change may touch; every file is still judged.
    #[arg(long, value_name = "glob")]
    scope: Vec<String>,
    /// Report suppressed findings at their own level, and refuse to guess.
    #[arg(long)]
    strict: bool,
    /// Write SARIF or a table, rather than choosing by what stdout is.
    #[arg(long, value_enum, value_name = "format")]
    format: Option<CheckFormat>,
    /// Read weed.toml from here instead of the repository root.
    #[arg(long, value_name = "path")]
    config: Option<PathBuf>,
    /// The message of the commit being prepared, for its Weed-allow trailers.
    #[arg(long, value_name = "path")]
    message_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct RulesArgs {
    /// Print the catalogue as a table or as json.
    #[arg(long, value_enum, value_name = "format", default_value_t = RulesFormat::Table)]
    format: RulesFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CheckFormat {
    Sarif,
    Table,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum RulesFormat {
    Table,
    Json,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return usage(&error),
    };

    let answer = match cli.command {
        Command::Check(args) => run_check(args),
        Command::Rules(args) => rules::run(match args.format {
            RulesFormat::Table => rules::Format::Table,
            RulesFormat::Json => rules::Format::Json,
        }),
    };

    write(&answer)
}

fn run_check(args: CheckArgs) -> Answer {
    let format = check::format_for(
        args.format.map(|format| match format {
            CheckFormat::Sarif => check::Format::Sarif,
            CheckFormat::Table => check::Format::Table,
        }),
        std::io::stdout().is_terminal(),
    );

    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            return Answer {
                code: EXIT_COULD_NOT_RUN,
                stdout: String::new(),
                stderr: vec![format!(
                    "weed could not read the directory it was called from: {error}. run it from a directory that exists."
                )],
            }
        }
    };

    check::run(&check::Request {
        cwd,
        base: args.base,
        staged: args.staged,
        scope: args.scope,
        strict: args.strict,
        format,
        config: args.config,
        message_file: args.message_file,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// `--help` and `--version` are answers, not failures. Anything else weed could
/// not parse is a run that never happened, so it fails closed with exit 3.
fn usage(error: &clap::Error) -> ExitCode {
    let _ = error.print();
    if error.use_stderr() {
        ExitCode::from(EXIT_COULD_NOT_RUN as u8)
    } else {
        ExitCode::SUCCESS
    }
}

fn write(answer: &Answer) -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    if !answer.stdout.is_empty() && stdout.write_all(answer.stdout.as_bytes()).is_err() {
        // Whoever asked stopped reading. The exit code still carries the verdict.
        return ExitCode::from(answer.code as u8);
    }
    let _ = stdout.flush();

    let mut stderr = std::io::stderr().lock();
    for line in &answer.stderr {
        let _ = writeln!(stderr, "{line}");
    }

    ExitCode::from(answer.code as u8)
}
