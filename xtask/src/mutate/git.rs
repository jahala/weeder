//! git, as the campaign asks it: one process, one answer, and a failure that
//! says which command refused rather than a guess about why.

use std::path::Path;
use std::process::Command;

/// Run a command and answer with its standard output, or with everything the
/// process said when it refused.
pub fn capture(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("git {}: {error}", arguments.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "git {} in {}: {}",
            arguments.join(" "),
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a command for what it does rather than for what it says.
pub fn run(root: &Path, arguments: &[&str]) -> Result<(), String> {
    capture(root, arguments).map(|_| ())
}

/// The lines of an answer, with the empty ones left out.
pub fn lines(root: &Path, arguments: &[&str]) -> Result<Vec<String>, String> {
    Ok(capture(root, arguments)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToString::to_string)
        .collect())
}
