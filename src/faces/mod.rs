//! The subcommands. A face gathers what it needs through the seams, asks core
//! for a judgement, and hands the caller an `Answer` to write out.

pub mod check;
pub mod guard;
pub mod rules;

use std::path::Path;

use crate::core::config::{parse_config, Config};
use crate::seams::fs;

/// The name of the file a repository states its law in.
pub const CONFIG_FILE: &str = "weed.toml";

/// This repository's configuration, or the defaults where it states none. A
/// config a caller pointed at and weed cannot read is a run that never happened;
/// a repository with no `weed.toml` simply takes the defaults.
pub fn read_config(root: &Path, from: Option<&Path>) -> Result<Config, String> {
    let path = match from {
        Some(path) => path.to_path_buf(),
        None => root.join(CONFIG_FILE),
    };
    let text = match from {
        Some(_) => Some(
            fs::read(&path)
                .map_err(|error| format!("{error} --config must name a file weed can read."))?,
        ),
        None => fs::read_if_present(&path).map_err(|error| error.to_string())?,
    };
    parse_config(text.as_deref()).map_err(|error| {
        format!(
            "{} is not valid: {error}. fix that key, or drop it for the default.",
            path.display()
        )
    })
}

/// What a face decided, and what it could not do. Writing to a stream and
/// leaving with a code is the caller's job, so a face stays testable whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub code: i32,
    pub stdout: String,
    /// One line per thing weed could not do, in the order it met them.
    pub stderr: Vec<String>,
}
