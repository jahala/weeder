//! Path globs, as `weeder.toml`'s `[scope] allow` and `--scope` spell them.
//!
//! `*` matches within one path segment, `**` crosses separators, `?` matches a
//! single character that is not a separator, and everything else is literal.
//! Paths are repository-relative and separated by `/`, so the matcher never
//! looks at the filesystem and never depends on the platform.

/// Whether a repository-relative path matches a glob.
pub fn matches(pattern: &str, path: &str) -> bool {
    match_from(pattern.as_bytes(), path.as_bytes())
}

/// Whether a path matches any of the globs. An empty list matches nothing, so a
/// caller that means "everything" passes a glob that says so.
pub fn matches_any(patterns: &[String], path: &str) -> bool {
    patterns.iter().any(|pattern| matches(pattern, path))
}

fn match_from(pattern: &[u8], path: &[u8]) -> bool {
    let Some(&first) = pattern.first() else {
        return path.is_empty();
    };
    match first {
        b'*' if pattern.get(1) == Some(&b'*') => match_crossing(&pattern[2..], path),
        b'*' => match_within(&pattern[1..], path),
        b'?' => {
            matches!(path.first(), Some(&byte) if byte != b'/')
                && match_from(&pattern[1..], &path[1..])
        }
        literal => path.first() == Some(&literal) && match_from(&pattern[1..], &path[1..]),
    }
}

/// `**` swallows any number of segments. `**/` also swallows none at all, so
/// `**/*.rs` matches `main.rs` as well as `src/core/main.rs`.
fn match_crossing(rest: &[u8], path: &[u8]) -> bool {
    if rest.first() == Some(&b'/') && match_from(&rest[1..], path) {
        return true;
    }
    (0..=path.len()).any(|index| match_from(rest, &path[index..]))
}

/// `*` stops at a separator, so it never reaches into a subdirectory.
fn match_within(rest: &[u8], path: &[u8]) -> bool {
    for index in 0..=path.len() {
        if match_from(rest, &path[index..]) {
            return true;
        }
        if path.get(index) == Some(&b'/') {
            return false;
        }
    }
    false
}
