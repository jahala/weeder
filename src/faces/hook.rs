//! `weed hook`, weed standing where the harness still has a choice.
//!
//! A harness writes a hook event as JSON on this face's stdin and reads weed's
//! answer back off its stdout. Two events matter. At a tool call that runs
//! `git commit`, weed judges the index and denies the call with the findings as
//! the reason; a commit that tells git to walk past its own hooks is denied
//! without judging anything, because nothing was offered for judgement. At the
//! end of a turn, weed judges the working tree and blocks the stop, so an agent
//! cannot declare itself done over a tree weed refuses. Every other event is
//! passed through in silence.
//!
//! Judgements run `--strict`, so an allowance an agent wrote for itself is
//! reported and not honoured, the same stance `weed guard` takes in git.
//!
//! A hook that could not judge refuses, at the block-level code, rather than
//! leaving with the could-not-run code: a harness reads only a refusal or
//! silence from a hook, and silence from a gate that never ran is a gate that
//! is not there.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::core::hook::{self, Ask, Harness, Refusal};
use crate::core::sarif::{EXIT_BLOCKED, EXIT_CLEAN};
use crate::faces::{check, Answer};
use crate::seams::git;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Where weed was called from, for an event that names no directory of its own.
    pub cwd: PathBuf,
    pub harness: Harness,
    /// The event as the harness wrote it on stdin.
    pub event: String,
    pub version: String,
}

pub fn run(request: &Request) -> Answer {
    let Ok(event) = serde_json::from_str::<Value>(&request.event) else {
        // A harness that wrote something weed cannot read is a harness weed
        // cannot answer, and the refusal it does understand is the safe one.
        return refuse(
            request,
            Refusal::Turn,
            &hook::unjudged(
                "weed could not read the event its harness wrote on stdin: it is not JSON.",
            ),
        );
    };

    let directory = match hook::working_directory(&event) {
        Some(cwd) => PathBuf::from(cwd),
        None => request.cwd.clone(),
    };

    match hook::read(request.harness, &event) {
        Ask::Pass => allowed(),
        Ask::Deaf { event } => noted(hook::note(request.harness, &event)),
        Ask::Bypass { flag } => refuse(request, Refusal::Tool, &hook::bypass_refused(&flag)),
        Ask::Commit { directories } => {
            let mut directory = directory;
            for named in directories {
                directory.push(named);
            }
            judge(
                request,
                Refusal::Tool,
                &directory,
                true,
                hook::commit_refused,
            )
        }
        Ask::Stop => judge(
            request,
            Refusal::Turn,
            &directory,
            false,
            hook::stop_refused,
        ),
    }
}

/// The index or the working tree, judged where the event says the work is, and
/// the answer written in the harness's shape where it blocks.
fn judge(
    request: &Request,
    refusal: Refusal,
    directory: &Path,
    staged: bool,
    reason: fn(&str) -> String,
) -> Answer {
    // A turn can end anywhere, and a command can name a directory that is no
    // repository. There is no diff to judge in either, and weed will not hold a
    // turn hostage to a place it was never asked about.
    let Ok(root) = git::repository_root(directory) else {
        return allowed();
    };

    let judgement = check::run(&check::Request {
        cwd: root,
        base: None,
        tip: None,
        staged,
        scope: Vec::new(),
        strict: true,
        format: check::Format::Table,
        config: None,
        message_file: None,
        version: request.version.clone(),
    });

    match judgement.code {
        EXIT_CLEAN => allowed(),
        EXIT_BLOCKED => refuse(request, refusal, &reason(&judgement.stdout)),
        _ => refuse(
            request,
            refusal,
            &hook::unjudged(&judgement.stderr.join(" ")),
        ),
    }
}

/// The refusal, in the harness's shape on stdout and in plain lines on stderr.
/// Which of the two a harness reads is the harness's business; both carry the
/// same reason, so the answer does not depend on knowing which.
fn refuse(request: &Request, refusal: Refusal, reason: &str) -> Answer {
    let answer = hook::answer(request.harness, refusal, reason);
    Answer {
        code: EXIT_BLOCKED,
        stdout: format!("{answer}\n"),
        stderr: reason.lines().map(ToString::to_string).collect(),
    }
}

/// One line, and no answer: there was nowhere to put one.
fn noted(note: String) -> Answer {
    Answer {
        code: EXIT_CLEAN,
        stdout: String::new(),
        stderr: vec![note],
    }
}

fn allowed() -> Answer {
    Answer {
        code: EXIT_CLEAN,
        stdout: String::new(),
        stderr: Vec::new(),
    }
}
