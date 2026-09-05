//! Reading a file, with the path in every error so a face can say which one.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsError {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not read {}: {}.", self.path, self.message)
    }
}

impl std::error::Error for FsError {}

/// A file's contents. A missing file is an error: the caller asked for this one.
pub fn read(path: &Path) -> Result<String, FsError> {
    std::fs::read_to_string(path).map_err(|error| FsError {
        path: path.display().to_string(),
        message: error.to_string(),
    })
}

/// A file's contents, or `None` where there is no such file. Anything else that
/// stops the read is still an error, so an unreadable file never reads as absent.
pub fn read_if_present(path: &Path) -> Result<Option<String>, FsError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(FsError {
            path: path.display().to_string(),
            message: error.to_string(),
        }),
    }
}

/// Whether a path is there at all.
pub fn exists(path: &Path) -> bool {
    path.exists()
}
