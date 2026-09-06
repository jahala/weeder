//! The subcommands. A face gathers what it needs through the seams, asks core
//! for a judgement, and hands the caller an `Answer` to write out.

pub mod bite;
pub mod check;
pub mod guard;
pub mod hook;
pub mod rules;
pub mod scan;

use std::path::Path;

use crate::core::change::{Change, Side};
use crate::core::classify::{classify_file, FileKind};
use crate::core::config::{parse_config, Config};
use crate::core::diff::FileDiff;
use crate::core::read::TestShape;
use crate::seams::{fs, git, reader};

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

/// How a face writes what it found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Sarif,
    Table,
}

/// SARIF when stdout is not a terminal, a table when it is, and `--format`
/// overrides both. A machine reading a pipe gets the log; a person gets the table.
pub fn format_for(requested: Option<Format>, stdout_is_terminal: bool) -> Format {
    match requested {
        Some(format) => format,
        None if stdout_is_terminal => Format::Table,
        None => Format::Sarif,
    }
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

/// Where a version of a file is to be found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A commit, by whatever name the caller gave it.
    Reference(String),
    /// The index: what a commit would carry.
    Index,
    /// The working tree, as it sits on disk.
    Tree,
}

/// Each changed file with both of its sides read: the text, what the path is,
/// and what the reader makes of the inside of it. This is the one place weed
/// touches a file for the rules, so a detector stays pure and testable whole.
pub fn gather(
    root: &Path,
    base: &str,
    after: &Source,
    judged: Vec<FileDiff>,
) -> Result<Vec<Change>, String> {
    judged
        .into_iter()
        .map(|diff| {
            let before = side(
                root,
                &Source::Reference(base.to_string()),
                diff.old_path.as_deref(),
            )?;
            let after = side(root, after, diff.new_path.as_deref())?;
            Ok(Change {
                diff,
                before,
                after,
            })
        })
        .collect()
}

/// One side of one file. A side with no file is nothing to read at all. A file
/// whose bytes are not text has no lines for a rule to judge, and still has a
/// path and a weight, which is what G2 asks about.
pub fn side(root: &Path, source: &Source, path: Option<&str>) -> Result<Side, String> {
    let Some(path) = path else {
        return Ok(Side::default());
    };
    let blob = match source {
        Source::Reference(reference) => git::file_at_ref(root, reference, path),
        Source::Index => git::file_in_index(root, path),
        Source::Tree => git::file_in_tree(root, path),
    }
    .map_err(|error| error.to_string())?;
    let Some(blob) = blob else {
        return Ok(Side::default());
    };
    let (size, binary) = (Some(blob.size()), blob.is_binary());
    let Some(content) = blob.text() else {
        return Ok(Side {
            classification: Some(classify_file(path, "")),
            size,
            binary,
            ..Side::default()
        });
    };

    let classification = classify_file(path, &content);
    let file = Path::new(path);
    let tests = if classification.kind == FileKind::Test || classification.has_inline_tests {
        reader::test_shape(file, &content)
    } else {
        TestShape::default()
    };
    // Every side is outlined, test file and production file alike: a rule that
    // asks what a rename did to a case, or which unit a double stands in for,
    // is asking about a declaration on whichever side of the suite it sits.
    let outline = reader::outline(file, &content);
    let imports = reader::imports(file, &content);
    Ok(Side {
        content: Some(content),
        classification: Some(classification),
        tests,
        outline,
        imports,
        size,
        binary,
    })
}
