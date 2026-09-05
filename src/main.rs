//! The command line. Every subcommand parses its flags here, asks its face for
//! an `Answer`, writes it, and leaves with the code the contract names: 0 clean
//! or warnings only, 2 at least one block-level result, 3 weed could not run.

use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use weed::core::hook::Harness;
use weed::core::sarif::EXIT_COULD_NOT_RUN;
use weed::faces::{check, guard, hook, rules, Answer};

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
    /// Put weed's judgement in git itself, through hooks git cannot be talked
    /// out of running.
    Guard(GuardArgs),
    /// Answer an agent harness's hook event, read as JSON on stdin, in the
    /// shape that harness reads its hooks' answers in.
    Hook(HarnessArgs),
    /// Print the rule catalogue and the level each rule carries.
    Rules(RulesArgs),
}

#[derive(Debug, Args)]
struct HarnessArgs {
    /// The harness whose event is on stdin.
    #[arg(value_enum, value_name = "harness")]
    harness: HookHarness,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum HookHarness {
    Claude,
    Gemini,
    Codex,
}

#[derive(Debug, Args)]
struct GuardArgs {
    #[command(subcommand)]
    command: GuardCommand,
}

#[derive(Debug, Subcommand)]
enum GuardCommand {
    /// Write the hooks and point core.hooksPath at them.
    Install(InstallArgs),
    /// Say whether every hook is still live, and name what is not.
    Status,
    /// Take the hooks away and put back the hooks path that was there.
    Uninstall,
    /// The pre-commit hook: judge the index. git runs this.
    PreCommit,
    /// The pre-push hook: judge what is being pushed, and keep protected
    /// branches from being rewritten. git runs this, and writes the refs on stdin.
    PrePush(HookArgs),
    /// The pre-rebase hook: refuse rewriting a protected branch. git runs this.
    PreRebase(HookArgs),
}

#[derive(Debug, Args)]
struct InstallArgs {
    /// Write the hooks here instead of .githooks.
    #[arg(long, value_name = "dir")]
    hooks_dir: Option<PathBuf>,
    /// A branch the hooks refuse to rewrite. Repeat it for more; leaving it out
    /// leaves the hooks reading weed.toml every time git runs them.
    #[arg(long, value_name = "branch")]
    protect: Vec<String>,
}

#[derive(Debug, Args)]
struct HookArgs {
    /// A branch the hook refuses to rewrite, as the installed bundle names them.
    #[arg(long, value_name = "branch")]
    protect: Vec<String>,
    /// What git puts after the hook's name.
    #[arg(value_name = "argument")]
    arguments: Vec<String>,
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
    install_panic_hook();
    #[cfg(debug_assertions)]
    fault_if_asked();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return usage(&error),
    };

    let answer = match cli.command {
        Command::Check(args) => run_check(args),
        Command::Guard(args) => run_guard(args),
        Command::Hook(args) => run_hook(args),
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
            return could_not_run(format!(
                "weed could not read the directory it was called from: {error}. run it from a directory that exists."
            ))
        }
    };

    check::run(&check::Request {
        cwd,
        base: args.base,
        tip: None,
        staged: args.staged,
        scope: args.scope,
        strict: args.strict,
        format,
        config: args.config,
        message_file: args.message_file,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

fn run_guard(args: GuardArgs) -> Answer {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            return could_not_run(format!(
                "weed could not read the directory it was called from: {error}. run it from a directory that exists."
            ))
        }
    };

    let command = match args.command {
        GuardCommand::Install(args) => {
            // The path weed is running from is what the hooks will name, and it
            // is resolved here rather than inside the face: asking the operating
            // system where this process came from is I/O like any other.
            let binary = match std::env::current_exe() {
                Ok(binary) => binary,
                Err(error) => {
                    return could_not_run(format!(
                        "weed could not find out where it is running from: {error}. a hook has to name the binary by its path, so install it from a weed on disk."
                    ))
                }
            };
            guard::Command::Install(guard::Install {
                hooks_dir: args.hooks_dir,
                protect: args.protect,
                binary,
            })
        }
        GuardCommand::Status => guard::Command::Status,
        GuardCommand::Uninstall => guard::Command::Uninstall,
        GuardCommand::PreCommit => guard::Command::PreCommit,
        GuardCommand::PrePush(args) => {
            let mut refs = String::new();
            if let Err(error) = std::io::stdin().read_to_string(&mut refs) {
                return could_not_run(format!(
                    "weed could not read the refs git writes on a pre-push hook's stdin: {error}. the push is refused rather than judged on nothing."
                ));
            }
            guard::Command::PrePush(guard::PrePush {
                protect: args.protect,
                arguments: args.arguments,
                refs,
            })
        }
        GuardCommand::PreRebase(args) => guard::Command::PreRebase(guard::PreRebase {
            protect: args.protect,
            arguments: args.arguments,
        }),
    };

    guard::run(&guard::Request {
        cwd,
        version: env!("CARGO_PKG_VERSION").to_string(),
        command,
    })
}

fn run_hook(args: HarnessArgs) -> Answer {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            return could_not_run(format!(
                "weed could not read the directory it was called from: {error}. run it from a directory that exists."
            ))
        }
    };

    let mut event = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut event) {
        return could_not_run(format!(
            "weed could not read the event its harness wrote on stdin: {error}. a hook is asked on stdin, and weed will not answer a question it did not hear."
        ));
    }

    hook::run(&hook::Request {
        cwd,
        harness: match args.harness {
            HookHarness::Claude => Harness::Claude,
            HookHarness::Gemini => Harness::Gemini,
            HookHarness::Codex => Harness::Codex,
        },
        event,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// A panic is a bug in weed, never a verdict, and a gate that dies part way
/// through judging still has to fail closed: exit 3, one line, and the line says
/// whose fault it is. The hook leaves through `exit` rather than by letting the
/// panic unwind, so a release build, which aborts on a panic and would
/// otherwise leave with a signal, leaves with the same code a debug build does.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic| {
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(
            stderr,
            "weed hit a bug and stopped rather than judge: {}. report it with the diff.",
            fault(panic)
        );
        let _ = stderr.flush();
        std::process::exit(EXIT_COULD_NOT_RUN);
    }));
}

/// What the panic said and where it said it, on one line. A payload written
/// across several lines is folded back onto one: whoever reads a hook's output
/// reads a line at a time, and a reason split over three of them is three
/// reasons as far as they can tell.
fn fault(panic: &std::panic::PanicHookInfo<'_>) -> String {
    let said = panic
        .payload_as_str()
        .unwrap_or("a panic that said nothing")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");
    match panic.location() {
        Some(location) => format!("{said} at {}:{}", location.file(), location.line()),
        None => said,
    }
}

/// The door a debug build leaves open so the panic hook can be proven on the
/// real binary rather than argued about: weed panics where it is told to, with
/// the words it is given. `cfg(debug_assertions)` keeps it out of a release
/// build, so the weed anyone installs has no way to be made to fall over from
/// outside it.
#[cfg(debug_assertions)]
fn fault_if_asked() {
    const ASKED: &str = "WEED_PANIC_FOR_TESTS";
    if let Some(reason) = std::env::var_os(ASKED) {
        panic!("{}", reason.to_string_lossy());
    }
}

/// A run that never happened: exit 3, and one line saying what stopped it.
fn could_not_run(reason: String) -> Answer {
    Answer {
        code: EXIT_COULD_NOT_RUN,
        stdout: String::new(),
        stderr: vec![reason],
    }
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
