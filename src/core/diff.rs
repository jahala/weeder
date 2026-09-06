#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Deleted,
    Modified,
    Renamed,
    Binary,
    ModeOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub change: ChangeKind,
    pub hunks: Vec<Hunk>,
    pub old_mode: Option<String>,
    pub new_mode: Option<String>,
}

impl FileDiff {
    /// What the file is called: where the change left it, or where it was
    /// before the change took it away.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.new_path.as_deref().or(self.old_path.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub section: Option<String>,
    pub lines: Vec<HunkLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HunkLine {
    pub kind: LineKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub text: String,
    pub no_newline: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffError {
    pub message: String,
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DiffError {}

pub fn parse_diff(input: &str) -> Result<Vec<FileDiff>, DiffError> {
    let lines: Vec<&str> = input.lines().collect();
    let mut files = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if !line.starts_with("diff --git ") {
            index += 1;
            continue;
        }

        let (old_path, new_path) = parse_diff_git_paths(line)?;
        let mut file = FileDiff {
            old_path: Some(old_path),
            new_path: Some(new_path),
            change: ChangeKind::Modified,
            hunks: Vec::new(),
            old_mode: None,
            new_mode: None,
        };
        let mut rename_from: Option<String> = None;
        let mut rename_to: Option<String> = None;
        let mut binary = false;
        let mut saw_hunk = false;
        let mut saw_mode = false;

        index += 1;
        while index < lines.len() && !lines[index].starts_with("diff --git ") {
            let current = lines[index];
            if let Some(rest) = current.strip_prefix("new file mode ") {
                file.change = ChangeKind::Added;
                file.old_path = None;
                file.new_mode = Some(rest.to_string());
                saw_mode = true;
            } else if let Some(rest) = current.strip_prefix("deleted file mode ") {
                file.change = ChangeKind::Deleted;
                file.new_path = None;
                file.old_mode = Some(rest.to_string());
                saw_mode = true;
            } else if let Some(rest) = current.strip_prefix("old mode ") {
                file.old_mode = Some(rest.to_string());
                saw_mode = true;
            } else if let Some(rest) = current.strip_prefix("new mode ") {
                file.new_mode = Some(rest.to_string());
                saw_mode = true;
            } else if let Some(rest) = current.strip_prefix("rename from ") {
                rename_from = Some(unquoted_path(rest));
                file.change = ChangeKind::Renamed;
            } else if let Some(rest) = current.strip_prefix("rename to ") {
                rename_to = Some(unquoted_path(rest));
                file.change = ChangeKind::Renamed;
            } else if current.starts_with("Binary files ")
                || current.starts_with("GIT binary patch")
            {
                binary = true;
            } else if current.starts_with("--- ") {
                diff_marker_path(current, "--- ").apply(&mut file.old_path);
            } else if current.starts_with("+++ ") {
                diff_marker_path(current, "+++ ").apply(&mut file.new_path);
            } else if current.starts_with("@@ ") {
                let (hunk, next_index) = parse_hunk(&lines, index)?;
                file.hunks.push(hunk);
                saw_hunk = true;
                index = next_index;
                continue;
            }
            index += 1;
        }

        if let Some(path) = rename_from {
            file.old_path = Some(path);
        }
        if let Some(path) = rename_to {
            file.new_path = Some(path);
        }
        if binary {
            file.change = ChangeKind::Binary;
        } else if saw_mode && !saw_hunk && matches!(file.change, ChangeKind::Modified) {
            file.change = ChangeKind::ModeOnly;
        }
        files.push(file);
    }

    Ok(files)
}

/// The two paths a `diff --git` line names. git writes each side as `a/<path>`
/// and `b/<path>`, and quotes a side whose path holds a quote, a backslash, a
/// control character or a byte outside ascii. It decides that for each side on
/// its own, so either, both or neither arrives quoted. An unquoted path may hold
/// spaces, which leaves the line ambiguous, and the split that gives both sides
/// the same path is the one git meant, because only a rename or a copy gives
/// them different ones, and those say so on their own lines further down.
fn parse_diff_git_paths(line: &str) -> Result<(String, String), DiffError> {
    let rest = line.strip_prefix("diff --git ").ok_or_else(|| DiffError {
        message: "diff header is malformed".to_string(),
    })?;
    let (old, new) = split_header_sides(rest).ok_or_else(|| DiffError {
        message: "diff header names no pair of paths".to_string(),
    })?;
    let old = old.strip_prefix("a/").ok_or_else(|| DiffError {
        message: "diff header old path is missing".to_string(),
    })?;
    let new = new.strip_prefix("b/").ok_or_else(|| DiffError {
        message: "diff header new path is missing".to_string(),
    })?;
    Ok((old.to_string(), new.to_string()))
}

/// The two sides of a `diff --git` line, still carrying their `a/` and `b/`.
fn split_header_sides(rest: &str) -> Option<(String, String)> {
    if rest.starts_with('"') {
        let (old, tail) = read_quoted(rest)?;
        let tail = tail.strip_prefix(' ')?;
        let new = if tail.starts_with('"') {
            read_quoted(tail)?.0
        } else {
            tail.to_string()
        };
        return Some((old, new));
    }
    // An unquoted path carries no quote of its own, a path with one in it is
    // always quoted, so the first `"` on the line opens the second side.
    if let Some(at) = rest.find(" \"") {
        if let Some((new, tail)) = read_quoted(&rest[at + 1..]) {
            if tail.is_empty() {
                return Some((rest[..at].to_string(), new));
            }
        }
    }
    // Both sides plain. The same path twice, with one space between them, is an
    // odd number of bytes with that space in the middle.
    let bytes = rest.as_bytes();
    if bytes.len() % 2 == 1 {
        let middle = bytes.len() / 2;
        // The halves are only whole where the middle byte is the space: a space
        // is one byte in utf-8 and never part of another character, so anywhere
        // else the middle could fall inside one.
        if bytes[middle] == b' ' {
            let (left, right) = (&rest[..middle], &rest[middle + 1..]);
            if let (Some(old), Some(new)) = (left.strip_prefix("a/"), right.strip_prefix("b/")) {
                if old == new {
                    return Some((left.to_string(), right.to_string()));
                }
            }
        }
    }
    // Two different plain paths, a rename or a copy. Where one of them holds
    // " b/" itself the split can only be guessed at, and the `rename from` and
    // `rename to` lines under the header put both paths right whatever is
    // guessed here.
    let mut offset = 0;
    while let Some(at) = rest[offset..].find(" b/") {
        let at = offset + at;
        if at > 2 && rest.len() > at + 3 && rest[..at].starts_with("a/") {
            return Some((rest[..at].to_string(), rest[at + 1..].to_string()));
        }
        offset = at + 1;
    }
    None
}

/// A path git wrote as a quoted C string: the bytes between two quotes, with
/// `\\`, `\"` and the usual control escapes spelled out, and every other byte
/// written as a backslash and three octal digits. The octal escapes carry the
/// bytes of the name, so they are gathered as bytes and read back as text at the
/// end. Returns the path and whatever followed the closing quote.
fn read_quoted(input: &str) -> Option<(String, &str)> {
    let mut bytes: Vec<u8> = Vec::new();
    let mut characters = input.char_indices();
    match characters.next() {
        Some((_, '"')) => {}
        _ => return None,
    }
    while let Some((index, character)) = characters.next() {
        match character {
            '"' => {
                return Some((
                    String::from_utf8_lossy(&bytes).into_owned(),
                    &input[index + 1..],
                ))
            }
            '\\' => {
                let (_, escape) = characters.next()?;
                match escape {
                    'a' => bytes.push(0x07),
                    'b' => bytes.push(0x08),
                    'f' => bytes.push(0x0c),
                    'n' => bytes.push(b'\n'),
                    'r' => bytes.push(b'\r'),
                    't' => bytes.push(b'\t'),
                    'v' => bytes.push(0x0b),
                    digit @ '0'..='7' => {
                        let mut value = digit as u32 - '0' as u32;
                        for _ in 0..2 {
                            value = value * 8 + characters.next()?.1.to_digit(8)?;
                        }
                        bytes.push(u8::try_from(value).ok()?);
                    }
                    other => push_character(&mut bytes, other),
                }
            }
            other => push_character(&mut bytes, other),
        }
    }
    None
}

fn push_character(bytes: &mut Vec<u8>, character: char) {
    let mut buffer = [0_u8; 4];
    bytes.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
}

/// What a `---` or `+++` line says about a path.
enum MarkerPath {
    Named(String),
    /// `/dev/null`: the side of the change where the file is not there at all.
    NoFile,
    /// A line weed could not read. The path the `diff --git` line already gave
    /// stands rather than being thrown away, a file with no path goes unjudged,
    /// which is the one direction a gate must not fail in.
    Unreadable,
}

impl MarkerPath {
    fn apply(self, path: &mut Option<String>) {
        match self {
            MarkerPath::Named(named) => *path = Some(named),
            MarkerPath::NoFile => *path = None,
            MarkerPath::Unreadable => {}
        }
    }
}

/// The path a `---` or `+++` line names. git ends the line with a tab where the
/// name holds a space, so the name and whatever follows it stay apart; an
/// unquoted name never holds a tab of its own, because a name with one in it is
/// quoted.
fn diff_marker_path(line: &str, prefix: &str) -> MarkerPath {
    let Some(value) = line.strip_prefix(prefix) else {
        return MarkerPath::Unreadable;
    };
    let value = if value.starts_with('"') {
        match read_quoted(value) {
            Some((path, _)) => path,
            None => return MarkerPath::Unreadable,
        }
    } else {
        value.trim_end_matches('\t').to_string()
    };
    if value == "/dev/null" {
        return MarkerPath::NoFile;
    }
    MarkerPath::Named(
        value
            .strip_prefix("a/")
            .or_else(|| value.strip_prefix("b/"))
            .unwrap_or(&value)
            .to_string(),
    )
}

/// The path a `rename from` or `rename to` line names, unquoted where git had
/// to quote it.
fn unquoted_path(raw: &str) -> String {
    if raw.starts_with('"') {
        if let Some((path, _)) = read_quoted(raw) {
            return path;
        }
    }
    raw.to_string()
}

fn parse_hunk(lines: &[&str], start: usize) -> Result<(Hunk, usize), DiffError> {
    let header = lines[start];
    let (old_start, old_count, new_start, new_count, section) = parse_hunk_header(header)?;
    // A header that starts a hunk at the end of the range git counts lines in
    // runs off that range on the first line of the body. git never writes such
    // a header, so weed does not have to make sense of one, it only has to
    // keep counting rather than wrap round to line one, which would put a
    // finding somewhere a reader would believe.
    let mut old_line = old_start;
    let mut new_line = new_start;
    let mut hunk_lines: Vec<HunkLine> = Vec::new();
    let mut index = start + 1;

    while index < lines.len()
        && !lines[index].starts_with("diff --git ")
        && !lines[index].starts_with("@@ ")
    {
        let line = lines[index];
        if line == "\\ No newline at end of file" {
            if let Some(previous) = hunk_lines.last_mut() {
                previous.no_newline = true;
            }
            index += 1;
            continue;
        }

        let Some(prefix) = line.chars().next() else {
            return Err(DiffError {
                message: "hunk line is empty".to_string(),
            });
        };
        let text = line.get(1..).unwrap_or_default().to_string();
        match prefix {
            ' ' => {
                hunk_lines.push(HunkLine {
                    kind: LineKind::Context,
                    old_line: Some(old_line),
                    new_line: Some(new_line),
                    text,
                    no_newline: false,
                });
                old_line = old_line.saturating_add(1);
                new_line = new_line.saturating_add(1);
            }
            '-' => {
                hunk_lines.push(HunkLine {
                    kind: LineKind::Removed,
                    old_line: Some(old_line),
                    new_line: None,
                    text,
                    no_newline: false,
                });
                old_line = old_line.saturating_add(1);
            }
            '+' => {
                hunk_lines.push(HunkLine {
                    kind: LineKind::Added,
                    old_line: None,
                    new_line: Some(new_line),
                    text,
                    no_newline: false,
                });
                new_line = new_line.saturating_add(1);
            }
            _ => break,
        }
        index += 1;
    }

    Ok((
        Hunk {
            old_start,
            old_count,
            new_start,
            new_count,
            section,
            lines: hunk_lines,
        },
        index,
    ))
}

fn parse_hunk_header(header: &str) -> Result<(u32, u32, u32, u32, Option<String>), DiffError> {
    let rest = header.strip_prefix("@@ -").ok_or_else(|| DiffError {
        message: "hunk header is malformed".to_string(),
    })?;
    let (old_range, rest) = rest.split_once(" +").ok_or_else(|| DiffError {
        message: "hunk old range is malformed".to_string(),
    })?;
    let (new_range, section) = rest.split_once(" @@").ok_or_else(|| DiffError {
        message: "hunk new range is malformed".to_string(),
    })?;
    let (old_start, old_count) = parse_range(old_range)?;
    let (new_start, new_count) = parse_range(new_range)?;
    let section = section.trim();
    Ok((
        old_start,
        old_count,
        new_start,
        new_count,
        (!section.is_empty()).then(|| section.to_string()),
    ))
}

fn parse_range(range: &str) -> Result<(u32, u32), DiffError> {
    if let Some((start, count)) = range.split_once(',') {
        Ok((
            parse_u32(start, "range start")?,
            parse_u32(count, "range count")?,
        ))
    } else {
        Ok((parse_u32(range, "range start")?, 1))
    }
}

fn parse_u32(value: &str, name: &str) -> Result<u32, DiffError> {
    value.parse::<u32>().map_err(|_| DiffError {
        message: format!("{name} is not a number"),
    })
}
