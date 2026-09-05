//! The exec seam: one command, run once, with a deadline it cannot outlive.
//!
//! git has a seam of its own because weed asks it a dozen different questions.
//! Everything else weed runs is a stranger — a CLI whose help a repository's
//! docs cite, a fetch of a registry's latest release — so it arrives here as a
//! program and an argument array, never a shell string, and it is given a
//! deadline. A command that will not finish is a command weed stops waiting on:
//! a scan is measured in milliseconds, and a hung child would spend the budget
//! of every rule behind it.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How often the wait looks at the child. Short enough that a fast command is
/// not held back by the poll, long enough that waiting is not a spin.
const POLL: Duration = Duration::from_millis(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecError {
    /// The program is not on PATH, or this machine refused to start it.
    Unavailable { program: String, message: String },
    /// The program started and was still running when the deadline passed.
    TimedOut { program: String, seconds: u64 },
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecError::Unavailable { program, message } => write!(
                f,
                "weed could not run {program}: {message}. install it, or put it on PATH."
            ),
            ExecError::TimedOut { program, seconds } => write!(
                f,
                "{program} was still running after {seconds}s and weed stopped waiting. a scan is measured in milliseconds, so the command it waits on has to answer in one."
            ),
        }
    }
}

impl std::error::Error for ExecError {}

/// What one command left behind. A command that ran and refused is an answer,
/// not a failure: the caller reads the code and decides what it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// One command, in a directory, with a deadline. Its output is read as lossy
/// text: a help listing or a registry's answer is read for the ascii shapes in
/// it, and a byte weed cannot read was never part of one.
pub fn run(
    directory: &Path,
    program: &str,
    arguments: &[&str],
    timeout: Duration,
) -> Result<Output, ExecError> {
    let mut child = Command::new(program)
        .current_dir(directory)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ExecError::Unavailable {
            program: program.to_string(),
            message: error.to_string(),
        })?;

    // The pipes are drained on threads of their own, so a command that writes
    // more than a pipe holds is never blocked waiting for a reader that is
    // itself waiting for the command to finish.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());

    let deadline = Instant::now() + timeout;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ExecError::TimedOut {
                    program: program.to_string(),
                    seconds: timeout.as_secs(),
                });
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(error) => {
                return Err(ExecError::Unavailable {
                    program: program.to_string(),
                    message: error.to_string(),
                })
            }
        }
    };

    Ok(Output {
        code,
        stdout: collect(stdout),
        stderr: collect(stderr),
    })
}

/// A pipe read to its end on a thread of its own.
fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    })
}

/// What a drained pipe held, as text. A thread that panicked read nothing,
/// which is the same answer as a command that wrote nothing.
fn collect(handle: std::thread::JoinHandle<Vec<u8>>) -> String {
    handle
        .join()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}
