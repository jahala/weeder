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
                rename_from = Some(rest.to_string());
                file.change = ChangeKind::Renamed;
            } else if let Some(rest) = current.strip_prefix("rename to ") {
                rename_to = Some(rest.to_string());
                file.change = ChangeKind::Renamed;
            } else if current.starts_with("Binary files ")
                || current.starts_with("GIT binary patch")
            {
                binary = true;
            } else if current.starts_with("--- ") {
                file.old_path = diff_marker_path(current, "--- ");
            } else if current.starts_with("+++ ") {
                file.new_path = diff_marker_path(current, "+++ ");
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

fn parse_diff_git_paths(line: &str) -> Result<(String, String), DiffError> {
    let rest = line.strip_prefix("diff --git ").ok_or_else(|| DiffError {
        message: "diff header is malformed".to_string(),
    })?;
    let mut parts = rest.split_whitespace();
    let old = parts
        .next()
        .and_then(|part| part.strip_prefix("a/"))
        .ok_or_else(|| DiffError {
            message: "diff header old path is missing".to_string(),
        })?;
    let new = parts
        .next()
        .and_then(|part| part.strip_prefix("b/"))
        .ok_or_else(|| DiffError {
            message: "diff header new path is missing".to_string(),
        })?;
    Ok((old.to_string(), new.to_string()))
}

fn diff_marker_path(line: &str, prefix: &str) -> Option<String> {
    let value = line.strip_prefix(prefix)?;
    match value {
        "/dev/null" => None,
        value => value
            .strip_prefix("a/")
            .or_else(|| value.strip_prefix("b/"))
            .or(Some(value))
            .map(ToString::to_string),
    }
}

fn parse_hunk(lines: &[&str], start: usize) -> Result<(Hunk, usize), DiffError> {
    let header = lines[start];
    let (old_start, old_count, new_start, new_count, section) = parse_hunk_header(header)?;
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
                old_line += 1;
                new_line += 1;
            }
            '-' => {
                hunk_lines.push(HunkLine {
                    kind: LineKind::Removed,
                    old_line: Some(old_line),
                    new_line: None,
                    text,
                    no_newline: false,
                });
                old_line += 1;
            }
            '+' => {
                hunk_lines.push(HunkLine {
                    kind: LineKind::Added,
                    old_line: None,
                    new_line: Some(new_line),
                    text,
                    no_newline: false,
                });
                new_line += 1;
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
