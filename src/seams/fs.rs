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
        Err(error) if is_nothing_to_read(&error, path) => Ok(None),
        Err(error) => Err(FsError {
            path: path.display().to_string(),
            message: error.to_string(),
        }),
    }
}

/// A file's bytes, or `None` where there is no such file. What those bytes are,
/// text a rule can read or a blob nobody can, is the caller's question to ask.
pub fn read_bytes_if_present(path: &Path) -> Result<Option<Vec<u8>>, FsError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if is_nothing_to_read(&error, path) => Ok(None),
        Err(error) => Err(FsError {
            path: path.display().to_string(),
            message: error.to_string(),
        }),
    }
}

/// A path with no file to read at it: nothing there, or a directory, a symlink
/// to one, a submodule, which has no lines and is not an error to be at.
///
/// Opening a directory as a file is refused with "is a directory" on unix and
/// with "access is denied" on Windows, so the second is read together with what
/// is at the path: a directory is nothing to read there too, and a file that is
/// really forbidden stays the error it is, so an unreadable file never reads as
/// absent.
fn is_nothing_to_read(error: &std::io::Error, path: &Path) -> bool {
    match error.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::IsADirectory => true,
        std::io::ErrorKind::PermissionDenied => path.is_dir(),
        _ => false,
    }
}

/// Whether a path is there at all.
pub fn exists(path: &Path) -> bool {
    path.exists()
}

/// A file written whole. The path travels in the error, so a face can say which
/// file it could not put down.
pub fn write(path: &Path, contents: &str) -> Result<(), FsError> {
    std::fs::write(path, contents).map_err(|error| FsError {
        path: path.display().to_string(),
        message: error.to_string(),
    })
}

/// A directory and every parent it still needs.
pub fn create_dir_all(path: &Path) -> Result<(), FsError> {
    std::fs::create_dir_all(path).map_err(|error| FsError {
        path: path.display().to_string(),
        message: error.to_string(),
    })
}

/// A file taken away. A file that is not there is already gone, so removing it
/// twice is not a failure.
pub fn remove_file(path: &Path) -> Result<(), FsError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(FsError {
            path: path.display().to_string(),
            message: error.to_string(),
        }),
    }
}

/// A directory taken away, and only when nothing is left in it. A directory that
/// still holds something is left alone: weeder removes what it wrote, not what it
/// found.
pub fn remove_dir_if_empty(path: &Path) -> Result<(), FsError> {
    match std::fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(FsError {
            path: path.display().to_string(),
            message: error.to_string(),
        }),
    }
}

// Whether a file runs is git's question and not weeder's, and git answers it
// differently on each platform, so both halves of the answer are written here.
//
// On unix git checks the execute bit before it calls a hook and walks past a
// hook it cannot run without a word, which is the one thing a gate must not do,
// so the bit is read and written. On Windows there is no such bit: NTFS keeps no
// mode, and Git for Windows takes `X_OK` out of the `access` call it makes
// before it looks for a hook, so any hook file that is there is a hook git runs,
// through the `sh` it ships, which reads the `#!/bin/sh` line install writes.
// There the question is whether the file is there, and there is no mode to write.

/// Whether a path is a file this machine will run. A hook git cannot execute is
/// a hook git walks past without a word, which is the one thing a gate must not do.
#[cfg(unix)]
pub fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// Whether a path is a file this machine will run. Windows keeps no execute bit
/// and git looks for none, so a hook file that is there is a hook git runs.
#[cfg(windows)]
pub fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// The mode a hook needs: everyone may read it and run it, its owner may rewrite it.
#[cfg(unix)]
pub fn make_executable(path: &Path) -> Result<(), FsError> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|error| {
        FsError {
            path: path.display().to_string(),
            message: error.to_string(),
        }
    })
}

/// The mode a hook needs on Windows, which is no mode at all: the file being
/// there is what makes git run it, so there is nothing to write and nothing that
/// can fail. The path is taken all the same, because the caller's question is
/// about that file and the answer has one shape on every platform.
#[cfg(windows)]
pub fn make_executable(_path: &Path) -> Result<(), FsError> {
    Ok(())
}

/// A path with every symlink and `..` resolved, or `None` where nothing is
/// there. Two spellings of one directory compare equal once both have been
/// through here.
pub fn canonical(path: &Path) -> Option<std::path::PathBuf> {
    std::fs::canonicalize(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A submodule is a directory where the index names a gitlink, and the
    /// working-tree reader meets it on every platform: unix refuses the open
    /// with "is a directory", Windows with "access is denied". Both are nothing
    /// to read, not an error.
    #[test]
    fn a_directory_is_nothing_to_read_on_every_platform() {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let directory = scratch.path().join("vendor").join("inner");
        std::fs::create_dir_all(&directory).expect("the directory");

        assert_eq!(read_bytes_if_present(&directory).expect("no error"), None);
        assert_eq!(read_if_present(&directory).expect("no error"), None);
        assert_eq!(
            read_bytes_if_present(&scratch.path().join("missing")).expect("no error"),
            None
        );
    }
}
