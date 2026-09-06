//! What one trial learned, and nothing about how it learned it.
//!
//! `bite` is the one dynamic judgement weed makes: it builds two states of a
//! repository and runs somebody else's test command over each. All of that is
//! I/O, and none of it belongs here. What arrives here is what came back, the
//! command's verdict on the test commit alone, and the change that commit
//! carries, so the rule that reads it stays a pure function of what it was
//! handed.

use crate::core::change::Change;

/// What a test command did with the state it was run over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Passed,
    Failed,
}

impl Verdict {
    /// A command's exit code read as a verdict. Every runner weed knows agrees
    /// on the one thing that matters: zero is a suite that passed.
    #[must_use]
    pub fn of(code: i32) -> Verdict {
        if code == 0 {
            Verdict::Passed
        } else {
            Verdict::Failed
        }
    }
}

/// The trial a bite-face rule judges: what the test commit changed, and what
/// the command did with that commit alone on the base.
#[derive(Debug, Clone, Copy)]
pub struct Trial<'a> {
    /// The files the test commit changed against the base, both sides read.
    pub tested: &'a [Change],
    /// What the test command did with the test commit alone applied.
    pub alone: Verdict,
}
