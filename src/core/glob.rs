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

/// Whether some path matches both globs. A setting written for one glob and
/// refused where it meets another needs to know whether the two can ever name
/// the same file, and listing the tree to find out would make the answer depend
/// on what the tree holds today.
///
/// Each glob is read as the small machine it is, a position in its pattern,
/// and the two are walked together one character at a time: a pair of
/// positions both patterns can reach on the same character is a pair a path
/// can reach, and both at their end is a path both match. The walk visits each
/// pair of positions once, so it ends.
pub fn overlap(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    let mut seen = std::collections::BTreeSet::new();
    let mut pending: Vec<(usize, usize)> = Vec::new();
    let mut reach = |a: usize, b: usize, pending: &mut Vec<(usize, usize)>| {
        for x in closure(left, a) {
            for y in closure(right, b) {
                if seen.insert((x, y)) {
                    pending.push((x, y));
                }
            }
        }
    };
    reach(0, 0, &mut pending);
    while let Some((a, b)) = pending.pop() {
        if a == left.len() && b == right.len() {
            return true;
        }
        if let (Some((one, after_one)), Some((other, after_other))) =
            (step(left, a), step(right, b))
        {
            if one.meets(other) {
                reach(after_one, after_other, &mut pending);
            }
        }
    }
    false
}

/// The characters one position of a glob consumes.
#[derive(Clone, Copy)]
enum Class {
    /// `**`: anything, a separator included.
    Any,
    /// `*` and `?`: anything but a separator.
    Segment,
    /// A literal byte.
    Byte(u8),
}

impl Class {
    /// Whether one character exists that both classes accept.
    fn meets(self, other: Class) -> bool {
        match (self, other) {
            (Class::Any, _) | (_, Class::Any) | (Class::Segment, Class::Segment) => true,
            (Class::Segment, Class::Byte(byte)) | (Class::Byte(byte), Class::Segment) => {
                byte != b'/'
            }
            (Class::Byte(one), Class::Byte(other)) => one == other,
        }
    }
}

/// What a glob can consume at one position, and where it stands afterwards.
/// A star stays where it is, because it may go on consuming.
fn step(pattern: &[u8], at: usize) -> Option<(Class, usize)> {
    match pattern.get(at)? {
        b'*' if pattern.get(at + 1) == Some(&b'*') => Some((Class::Any, at)),
        b'*' => Some((Class::Segment, at)),
        b'?' => Some((Class::Segment, at + 1)),
        &byte => Some((Class::Byte(byte), at + 1)),
    }
}

/// Every position a glob stands at without consuming anything from here: a
/// star may match nothing, and `**/` may swallow its separator too, as
/// [`matches`] lets it.
fn closure(pattern: &[u8], at: usize) -> Vec<usize> {
    let mut found = vec![at];
    let mut index = 0;
    while let Some(&position) = found.get(index) {
        index += 1;
        let next: &[usize] = match (pattern.get(position), pattern.get(position + 1)) {
            (Some(b'*'), Some(b'*')) if pattern.get(position + 2) == Some(&b'/') => {
                &[position + 2, position + 3]
            }
            (Some(b'*'), Some(b'*')) => &[position + 2],
            (Some(b'*'), _) => &[position + 1],
            _ => &[],
        };
        for &reached in next {
            if !found.contains(&reached) {
                found.push(reached);
            }
        }
    }
    found
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
