//! The subcommands. A face gathers what it needs through the seams, asks core
//! for a judgement, and hands the caller an `Answer` to write out.

pub mod check;
pub mod rules;

/// What a face decided, and what it could not do. Writing to a stream and
/// leaving with a code is the caller's job, so a face stays testable whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub code: i32,
    pub stdout: String,
    /// One line per thing weed could not do, in the order it met them.
    pub stderr: Vec<String>,
}
