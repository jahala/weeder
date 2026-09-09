//! The subcommands. A face gathers what it needs through the seams, asks core
//! for a judgement, and hands the caller an `Answer` to write out.

pub mod bite;
pub mod check;
pub mod guard;
pub mod hook;
pub mod rules;
pub mod scan;

use std::path::Path;

use crate::core::change::{self, Change, Side};
use crate::core::classify::{classify_file, FileKind};
use crate::core::config::{parse_config, Config};
use crate::core::diff::FileDiff;
use crate::core::read::TestShape;
use crate::core::syntax::Mask;
use crate::seams::git::Blob;
use crate::seams::{fs, git, reader};

/// The name of the file a repository states its law in.
pub const CONFIG_FILE: &str = "weeder.toml";

/// This repository's configuration, or the defaults where it states none. A
/// config a caller pointed at and weeder cannot read is a run that never happened;
/// a repository with no `weeder.toml` simply takes the defaults.
pub fn read_config(root: &Path, from: Option<&Path>) -> Result<Config, String> {
    let path = match from {
        Some(path) => path.to_path_buf(),
        None => root.join(CONFIG_FILE),
    };
    let text = match from {
        Some(_) => Some(
            fs::read(&path)
                .map_err(|error| format!("{error} --config must name a file weeder can read."))?,
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
    /// One line per thing weeder could not do, in the order it met them.
    pub stderr: Vec<String>,
}

/// Whether a run judges the files git has never been told about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Untracked {
    Include,
    Exclude,
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
/// and what the reader makes of the inside of it. This is the one place weeder
/// touches a file for the rules, so a detector stays pure and testable whole.
///
/// Both sides of every file are read in one question to the seam rather than
/// one question per file: a fifty-file change costs a handful of processes
/// instead of a hundred, and a person waiting on a commit feels the difference.
pub fn gather(
    root: &Path,
    base: &str,
    after: &Source,
    judged: Vec<FileDiff>,
) -> Result<Vec<Change>, String> {
    let before_paths: Vec<Option<&str>> =
        judged.iter().map(|diff| diff.old_path.as_deref()).collect();
    let after_paths: Vec<Option<&str>> =
        judged.iter().map(|diff| diff.new_path.as_deref()).collect();
    let before = sides(root, &Source::Reference(base.to_string()), &before_paths)?;
    let after = sides(root, after, &after_paths)?;
    Ok(judged
        .into_iter()
        .zip(before)
        .zip(after)
        .map(|((diff, before), after)| Change {
            diff,
            before,
            after,
        })
        .collect())
}

/// One side of each of these files, in the order they were asked for. A side
/// with no file is nothing to read at all, and keeps its place in the answer.
fn sides(root: &Path, source: &Source, paths: &[Option<&str>]) -> Result<Vec<Side>, String> {
    let named: Vec<&str> = paths.iter().flatten().copied().collect();
    let mut read = blobs(root, source, &named)?.into_iter();
    paths
        .iter()
        .map(|path| match path {
            None => Ok(Side::default()),
            Some(path) => {
                let blob = read
                    .next()
                    .ok_or_else(|| format!("git answered for fewer files than weeder asked about, and {path} was one of them. report it with the change that caused it."))?;
                Ok(blob.map_or_else(Side::default, |blob| side(path, &blob)))
            }
        })
        .collect()
}

/// The bytes each path holds where the caller is looking. The index and a ref
/// are questions for git, and both answer for as many paths as they are asked
/// about at once; the working tree is a question for the filesystem, which has
/// no process to start.
fn blobs(root: &Path, source: &Source, paths: &[&str]) -> Result<Vec<Option<Blob>>, String> {
    match source {
        Source::Reference(reference) => git::files_at_ref(root, reference, paths),
        Source::Index => git::files_in_index(root, paths),
        Source::Tree => paths
            .iter()
            .map(|path| git::file_in_tree(root, path))
            .collect(),
    }
    .map_err(|error| error.to_string())
}

/// One side of one file, as the bytes git handed over. A file whose bytes are
/// not text has no lines for a rule to judge, and still has a path and a
/// weight, which is what G2 asks about.
pub(crate) fn side(path: &str, blob: &Blob) -> Side {
    let (size, binary) = (Some(blob.size()), blob.is_binary());
    let Some(content) = blob.text() else {
        return Side {
            classification: Some(classify_file(path, "")),
            size,
            binary,
            ..Side::default()
        };
    };

    let classification = classify_file(path, &content);
    let file = Path::new(path);
    // The one scan of the file's bytes: what every rule that asks whether a
    // line is code, a comment or a literal reads, so none of them scans it
    // again. A file too large to parse is still a file whose lines are judged.
    let syntax = Mask::of(classification.lang, &content);
    // A file past the weight anyone reads is weighed and counted rather than
    // parsed. G2 reports the file itself, and asking the parser to outline a
    // blob nobody will open costs more than every rule in the run together.
    if !change::reads_as_code(size) {
        return Side {
            content: Some(content),
            classification: Some(classification),
            size,
            binary,
            syntax,
            ..Side::default()
        };
    }
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
    Side {
        content: Some(content),
        classification: Some(classification),
        tests,
        outline,
        imports,
        size,
        binary,
        syntax,
    }
}
