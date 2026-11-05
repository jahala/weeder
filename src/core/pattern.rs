//! The path patterns `[scope] exclude` and `[[overrides]]` are written in.
//!
//! They read the way a `.gitignore` line does, anchored at the repository root:
//! `test/fixtures/` names a directory and everything under it, `docs/dogfood/**`
//! the same by its contents, `**/*.snap` a kind of file at any depth, and a
//! pattern that names a directory covers every path inside it. `*`, `**` and
//! `?` mean what they mean in [`crate::core::glob`]. Paths are repository
//! relative and separated by `/`, so nothing here reads a disk.
//!
//! Some patterns are refused, because a gate that can be told not to read its
//! own law is no gate: an absolute path, a step out with `..`, a pattern that
//! matches everything, and one that can name a guardrail file.

use crate::core::glob;

/// Whether a pattern covers a repository-relative path: it matches the path,
/// or it matches a directory the path sits in.
#[must_use]
pub fn covers(pattern: &str, path: &str) -> bool {
    let (glob, directory) = parts(pattern);
    if !directory && glob::matches(glob, path) {
        return true;
    }
    path.match_indices('/')
        .any(|(at, _)| glob::matches(glob, &path[..at]))
}

/// Whether some path can be covered by both patterns. Two settings that
/// disagree are only a conflict where a file can be under both.
#[must_use]
pub fn meet(left: &str, right: &str) -> bool {
    forms(left)
        .iter()
        .any(|one| forms(right).iter().any(|other| glob::overlap(one, other)))
}

/// Whether a pattern can cover a path a glob names.
#[must_use]
pub fn reaches(pattern: &str, target: &str) -> bool {
    forms(pattern)
        .iter()
        .any(|form| glob::overlap(form, target))
}

/// How specific a pattern is: the literal characters it holds once a trailing
/// `/` or `/**` is read past, since `docs`, `docs/` and `docs/**` cover the
/// same paths. Of two patterns that cover a file, the one with more of its
/// path written out is the one meant for it.
#[must_use]
pub fn specificity(pattern: &str) -> usize {
    let (mut glob, _) = parts(pattern);
    while let Some(shorter) = glob.strip_suffix("/**") {
        glob = shorter.trim_end_matches('/');
    }
    glob.bytes()
        .filter(|byte| !matches!(byte, b'*' | b'?'))
        .count()
}

/// Why a pattern cannot be written, or `None` where it can. `guardrails` are
/// the globs of the files that hold weeder's law or run its checks. Every
/// refusal names the pattern and the way forward, because the caller has a
/// list and a refusal that does not say which line was wrong sends the reader
/// back to guess.
#[must_use]
pub fn refusal(pattern: &str, guardrails: &[&str]) -> Option<String> {
    let (glob, _) = parts(pattern);
    if pattern.trim().is_empty() {
        return Some(format!(
            "`{pattern}` names no path. write the directory or files it means, from the repository root, such as `test/fixtures/`"
        ));
    }
    if glob.is_empty()
        || glob
            .split('/')
            .all(|segment| segment.chars().all(|c| c == '*'))
    {
        return Some(format!(
            "`{pattern}` matches every path, and a gate told to read nothing is no gate. name the directories or files it is meant for, such as `test/fixtures/`; a level for every path belongs under [rules]"
        ));
    }
    if absolute(pattern) {
        return Some(format!(
            "`{pattern}` is an absolute path, and these patterns are read from the repository root. write it relative to the root, such as `test/fixtures/`"
        ));
    }
    if pattern.split(['/', '\\']).any(|segment| segment == "..") {
        return Some(format!(
            "`{pattern}` steps out with `..`, and a pattern here may only name paths inside the repository. write it from the root, without `..`"
        ));
    }
    if let Some(guardrail) = guardrails.iter().find(|target| names(pattern, target)) {
        let guardrail = shown(guardrail);
        return Some(format!(
            "`{pattern}` can cover {guardrail}, which holds weeder's law or runs its checks, and a gate told not to read its own law is no gate. narrow the pattern so it leaves {guardrail} out"
        ));
    }
    None
}

/// Whether a pattern names a guardrail: it covers the guardrail's own place,
/// `weeder.toml`, `.github/workflows`, or it is written from inside that place
/// and can reach the files the guardrail holds there.
///
/// A pattern that reaches a guardrail only because a leading `**` or `*`
/// stands where the guardrail's directory is, `**/testdata/` against
/// `.githooks/testdata/`, does not name it. Refusing those would refuse every
/// pattern that looks for a directory by name at any depth, and they need no
/// refusal: a guardrail file is never excluded, whatever covers it, which
/// [`crate::core::exclude`] holds to.
fn names(pattern: &str, guardrail: &str) -> bool {
    let place = fixed(guardrail);
    if place.is_empty() {
        return reaches(pattern, guardrail);
    }
    if covers(pattern, &place) {
        return true;
    }
    let own = fixed(parts(pattern).0);
    (own == place || own.starts_with(&format!("{place}/"))) && reaches(pattern, guardrail)
}

/// The leading segments of a glob that hold no wildcard: the place it is
/// written from.
fn fixed(glob: &str) -> String {
    glob.split('/')
        .take_while(|segment| !segment.contains(['*', '?']))
        .collect::<Vec<&str>>()
        .join("/")
}

/// A pattern as the glob it matches with, and whether it was written as a
/// directory, with a trailing `/`.
fn parts(pattern: &str) -> (&str, bool) {
    (pattern.trim_end_matches('/'), pattern.ends_with('/'))
}

/// The globs whose union is everything a pattern covers: the paths it matches,
/// unless it names a directory only, and everything under what it matches.
fn forms(pattern: &str) -> Vec<String> {
    let (glob, directory) = parts(pattern);
    let mut forms = vec![format!("{glob}/**")];
    if !directory {
        forms.push(glob.to_string());
    }
    forms
}

/// Whether a pattern names a place outside the repository by its own spelling:
/// a root, a drive, a home directory.
fn absolute(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    pattern.starts_with(['/', '\\', '~'])
        || (bytes.len() > 1 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
}

/// A guardrail glob the way a reader names the place: `.githooks/` rather than
/// `.githooks/**`.
fn shown(guardrail: &str) -> String {
    match guardrail.strip_suffix("/**") {
        Some(directory) => format!("`{directory}/`"),
        None => format!("`{guardrail}`"),
    }
}
