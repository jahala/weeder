//! Core, handed what nobody wrote on purpose.
//!
//! `parse_diff` and `classify_file` are the two doors every face pushes its
//! input through, and neither may panic: a gate that dies on a shape it did not
//! expect is a gate that stopped judging without saying so. The property is
//! always the same, the call returns, with an answer or with an error, and the
//! answer it returns is consistent with itself.
//!
//! The corpus is real. A git repository is built with the shapes that hurt ,
//! renames, a symlink, a binary file, CRLF, a mode change, a file with no
//! newline at the end, a deletion, and git's own diffs of it are what the
//! mutations start from. Bytes weeder cannot read as utf-8 arrive as the seam
//! hands them over: lossy text, with the replacement character where the byte
//! was.

mod common;

use std::sync::OnceLock;

use proptest::prelude::*;

use common::Repo;
use weeder::core::diff::{FileDiff, LineKind};
use weeder::core::{classify_file, parse_diff, FileKind, Lang};

proptest! {
    /// Bytes with no shape at all. Most of these carry no `diff --git` line and
    /// parse to nothing, which is the right answer; the point is that none of
    /// them stops weeder.
    #[test]
    fn arbitrary_bytes_are_judged_or_refused(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = judge(&String::from_utf8_lossy(&bytes));
    }

    /// Text assembled from the pieces a diff is made of. Random bytes almost
    /// never reach the hunk parser; these do.
    #[test]
    fn diff_shaped_text_is_judged_or_refused(text in diff_shaped_text()) {
        let _ = judge(&text);
    }

    /// Real diffs, damaged. Truncated part way through a hunk, a header with a
    /// field taken out, counts that lie, line ends turned to CRLF, bytes spliced
    /// in that are not utf-8.
    #[test]
    fn a_damaged_real_diff_is_judged_or_refused(damaged in damaged_diff()) {
        let _ = judge(&damaged);
    }

    /// Any path, any content. `classify_file` reads both and must answer for
    /// every pair, including the paths an operating system would refuse.
    #[test]
    fn any_path_and_content_are_classified(
        path in ".{0,200}",
        content in proptest::collection::vec(any::<u8>(), 0..2048),
    ) {
        classify(&path, &String::from_utf8_lossy(&content));
    }

    /// Paths built out of the pieces that decide a classification, so the
    /// interesting combinations come up rather than being waited for.
    #[test]
    fn file_shaped_paths_are_classified(path in file_shaped_path(), content in content_text()) {
        classify(&path, &content);
    }
}

/// Numbers a hunk header can carry that weeder still has to read. The last two are
/// the end of the range a line number lives in: counting on from there leaves it.
const PARSEABLE_NUMBERS: &[&str] = &["0", "1", "7", "4294967294", "4294967295"];

/// Numbers a hunk header can carry that weeder cannot read at all, too large for
/// the range, negative, empty, or not a number.
const UNREADABLE_NUMBERS: &[&str] = &[
    "4294967296",
    "99999999999999999999",
    "-1",
    "",
    "x",
    "1,",
    ",1",
];

/// A number for a hunk header, weighted towards the ones that parse. A header
/// weeder refuses tells nothing about what happens after it, so most of them have
/// to get through.
fn hostile_number() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        3 => proptest::sample::select(PARSEABLE_NUMBERS),
        1 => proptest::sample::select(UNREADABLE_NUMBERS),
    ]
}

/// The whole diff at once, no fixture is this big, and weeder must not care.
#[test]
fn a_huge_diff_is_judged_without_falling_over() {
    let mut huge = String::from("diff --git a/src/huge.ts b/src/huge.ts\n");
    huge.push_str("--- a/src/huge.ts\n+++ b/src/huge.ts\n");
    huge.push_str("@@ -1,1 +1,400000 @@\n");
    huge.push_str("-const before = 1;\n");
    for index in 0..400_000 {
        huge.push_str(&format!("+const line{index} = {index};\n"));
    }
    let files = judge(&huge).expect("a large but well-formed diff parses");
    assert_eq!(files.len(), 1, "one file");
    assert_eq!(
        files[0].hunks[0].lines.len(),
        400_001,
        "every line of the hunk is read"
    );
}

/// A hunk that starts at the last line a `u32` can name. Counting on from there
/// leaves the range, and weeder has to answer rather than come apart.
#[test]
fn a_hunk_numbered_at_the_end_of_the_range_is_judged() {
    let diff = [
        "diff --git a/src/a.ts b/src/a.ts",
        "--- a/src/a.ts",
        "+++ b/src/a.ts",
        "@@ -4294967295,3 +4294967295,3 @@",
        " context",
        " more context",
        "+added",
        "",
    ]
    .join("\n");
    let files = judge(&diff).expect("a hunk at the end of the range parses");
    let lines = &files[0].hunks[0].lines;
    assert_eq!(lines.len(), 3, "every line of the hunk is read");
    assert!(
        lines
            .iter()
            .all(|line| line.new_line.is_none_or(|number| number == u32::MAX)),
        "a line past the end of the range stays at the end of it rather than wrapping"
    );
}

/// A file whose lines have no ending at all: one line, megabytes long.
#[test]
fn a_diff_with_no_line_ends_is_judged() {
    let single = format!(
        "diff --git a/a b/a\n@@ -1 +1 @@\n+{}",
        "a".repeat(2_000_000)
    );
    judge(&single).expect("one very long added line parses");
}

/// The property, in full: the call comes back, an error says something, and an
/// answer agrees with itself. Parsing the same bytes twice gives the same
/// answer, which is the order law's foundation.
fn judge(input: &str) -> Result<Vec<FileDiff>, weeder::core::DiffError> {
    let parsed = parse_diff(input);
    assert_eq!(
        parse_diff(input),
        parsed,
        "parsing the same bytes twice must give the same answer"
    );

    match &parsed {
        Err(error) => assert!(
            !error.message.trim().is_empty(),
            "a refusal has to say what it could not read"
        ),
        Ok(files) => {
            for file in files {
                for hunk in &file.hunks {
                    let mut previous_old = 0;
                    let mut previous_new = 0;
                    for line in &hunk.lines {
                        assert_eq!(
                            line.old_line.is_some(),
                            matches!(line.kind, LineKind::Context | LineKind::Removed),
                            "an old line number belongs to the lines the old file had"
                        );
                        assert_eq!(
                            line.new_line.is_some(),
                            matches!(line.kind, LineKind::Context | LineKind::Added),
                            "a new line number belongs to the lines the new file has"
                        );
                        if let Some(number) = line.old_line {
                            assert!(
                                number >= previous_old,
                                "old line numbers never go backwards"
                            );
                            previous_old = number;
                        }
                        if let Some(number) = line.new_line {
                            assert!(
                                number >= previous_new,
                                "new line numbers never go backwards"
                            );
                            previous_new = number;
                        }
                    }
                }
            }
        }
    }
    parsed
}

/// The classification property: the call comes back, it comes back the same way
/// twice, and what it says about the file holds together.
fn classify(path: &str, content: &str) {
    let classification = classify_file(path, content);
    assert_eq!(
        classify_file(path, content),
        classification,
        "classifying the same file twice must give the same answer"
    );
    if classification.has_inline_tests {
        assert_eq!(
            classification.kind,
            FileKind::Prod,
            "only a production file carries inline tests; a test file is all tests"
        );
        assert_eq!(
            classification.lang,
            Lang::Rust,
            "inline tests are a rust shape"
        );
    }
}

/// Text made of the pieces a unified diff is built from, whole files with
/// hunks under them, hunks on their own, and loose lines, so the parser meets
/// hunks that follow a header as often as hunks that do not, headers with no
/// file, bodies with no hunk, and every other half-formed shape. Loose lines
/// alone almost never reach the line counter inside a hunk; a header with a
/// body under it reaches it every time, which is where the numbers get large.
fn diff_shaped_text() -> impl Strategy<Value = String> {
    let header = prop_oneof![
        4 => Just("diff --git a/src/a.ts b/src/a.ts".to_string()),
        4 => Just("diff --git a/a b/b".to_string()),
        1 => Just("diff --git".to_string()),
        1 => Just("diff --git a/only".to_string()),
        1 => Just("diff --git \"a/a\\\"q\\\".ts\" \"b/a\\\"q\\\".ts\"".to_string()),
        1 => Just("diff --git a/a file b/a file".to_string()),
    ];
    let attribute = prop_oneof![
        Just("new file mode 100644".to_string()),
        Just("deleted file mode 100644".to_string()),
        Just("old mode 100644".to_string()),
        Just("new mode 100755".to_string()),
        Just("rename from src/a.ts".to_string()),
        Just("rename to src/b.ts".to_string()),
        Just("Binary files a/x and b/x differ".to_string()),
        Just("GIT binary patch".to_string()),
        Just("index 1111111..2222222 100644".to_string()),
        Just("--- a/src/a.ts".to_string()),
        Just("--- /dev/null".to_string()),
        Just("+++ b/src/a.ts".to_string()),
        Just("+++ /dev/null".to_string()),
    ];
    let hunk_header = prop_oneof![
        1 => Just("@@".to_string()),
        1 => Just("@@ @@".to_string()),
        4 => (hostile_number(), hostile_number(), hostile_number(), hostile_number())
            .prop_map(|(a, b, c, d)| format!("@@ -{a},{b} +{c},{d} @@ section")),
        4 => (hostile_number(), hostile_number())
            .prop_map(|(a, c)| format!("@@ -{a} +{c} @@")),
    ];
    let body_line = prop_oneof![
        3 => Just(" context".to_string()),
        3 => Just("+added".to_string()),
        3 => Just("-removed".to_string()),
        1 => Just("\\ No newline at end of file".to_string()),
        1 => Just(String::new()),
        1 => ".{0,40}".prop_map(|text: String| text),
    ];

    let hunk = (
        hunk_header,
        proptest::collection::vec(body_line.clone(), 0..12),
    )
        .prop_map(|(header, body)| {
            let mut lines = vec![header];
            lines.extend(body);
            lines
        });
    let file = (
        header,
        proptest::collection::vec(attribute, 0..5),
        proptest::collection::vec(hunk.clone(), 0..3),
    )
        .prop_map(|(header, attributes, hunks)| {
            let mut lines = vec![header];
            lines.extend(attributes);
            lines.extend(hunks.concat());
            lines
        });
    let loose = body_line.prop_map(|line| vec![line]);

    let chunk = prop_oneof![4 => file, 1 => hunk, 1 => loose];
    proptest::collection::vec(chunk, 0..8).prop_map(|chunks| chunks.concat().join("\n"))
}

/// The pieces a path is judged on: a directory that means tests, a name that
/// means generated, an extension that names a language, and the odd shapes an
/// agent can still write down.
fn file_shaped_path() -> impl Strategy<Value = String> {
    let segment = prop_oneof![
        Just("src".to_string()),
        Just("tests".to_string()),
        Just("__tests__".to_string()),
        Just("benches".to_string()),
        Just("dist".to_string()),
        Just(".github".to_string()),
        Just("workflows".to_string()),
        Just("..".to_string()),
        Just(".".to_string()),
        Just(String::new()),
        Just("a directory with spaces".to_string()),
        "[^/]{0,12}".prop_map(|text: String| text),
    ];
    let name = prop_oneof![
        Just("parser.ts".to_string()),
        Just("parser.test.ts".to_string()),
        Just("test_parser.py".to_string()),
        Just("parser_test.go".to_string()),
        Just("mod.rs".to_string()),
        Just("tests.rs".to_string()),
        Just("weeder.toml".to_string()),
        Just("Cargo.toml".to_string()),
        Just("bundle.min.js".to_string()),
        Just(".gitignore".to_string()),
        Just("".to_string()),
        Just(".".to_string()),
        Just("a\"quoted\".ts".to_string()),
        "[^/]{0,20}".prop_map(|text: String| text),
    ];
    (proptest::collection::vec(segment, 0..5), name).prop_map(|(segments, name)| {
        let mut path = segments.join("/");
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(&name);
        path
    })
}

fn content_text() -> impl Strategy<Value = String> {
    let line = prop_oneof![
        Just("#[cfg(test)]".to_string()),
        Just("#[test]".to_string()),
        Just("// Generated by the thing that generates it".to_string()),
        Just("// DO NOT EDIT".to_string()),
        Just("auto-generated".to_string()),
        ".{0,40}".prop_map(|text: String| text),
    ];
    proptest::collection::vec(line, 0..20).prop_map(|lines| lines.join("\n"))
}

/// A real diff with something done to it.
fn damaged_diff() -> impl Strategy<Value = String> {
    (
        proptest::sample::select(corpus().to_vec()),
        proptest::collection::vec(mutation(), 0..6),
    )
        .prop_map(|(diff, mutations)| {
            mutations
                .into_iter()
                .fold(diff, |text, mutation| mutation.apply(&text))
        })
}

#[derive(Debug, Clone)]
enum Mutation {
    /// Cut the diff off part way through, as a killed git would.
    Truncate(usize),
    DropLine(usize),
    DuplicateLine(usize),
    ReplaceLine(usize, String),
    /// Rewrite every hunk header so its numbers say something untrue.
    LyingCounts(String, String, String, String),
    Crlf,
    /// Put bytes in that are not utf-8, and read the result back the way the
    /// seam does.
    Splice(usize, Vec<u8>),
    /// Every line end gone, so the whole diff is one line.
    JoinLines,
}

impl Mutation {
    fn apply(self, text: &str) -> String {
        match self {
            Mutation::Truncate(at) => {
                let boundary = boundary(text, at);
                text[..boundary].to_string()
            }
            Mutation::DropLine(at) => with_lines(text, |lines| {
                if !lines.is_empty() {
                    lines.remove(at % lines.len());
                }
            }),
            Mutation::DuplicateLine(at) => with_lines(text, |lines| {
                if !lines.is_empty() {
                    let index = at % lines.len();
                    let line = lines[index].clone();
                    lines.insert(index, line);
                }
            }),
            Mutation::ReplaceLine(at, replacement) => with_lines(text, |lines| {
                if !lines.is_empty() {
                    let index = at % lines.len();
                    lines[index] = replacement.clone();
                }
            }),
            Mutation::LyingCounts(a, b, c, d) => with_lines(text, |lines| {
                for line in lines.iter_mut() {
                    if line.starts_with("@@ ") {
                        *line = format!("@@ -{a},{b} +{c},{d} @@");
                    }
                }
            }),
            Mutation::Crlf => text.replace('\n', "\r\n"),
            Mutation::Splice(at, bytes) => {
                let boundary = boundary(text, at);
                let mut spliced = text.as_bytes()[..boundary].to_vec();
                spliced.extend_from_slice(&bytes);
                spliced.extend_from_slice(&text.as_bytes()[boundary..]);
                String::from_utf8_lossy(&spliced).into_owned()
            }
            Mutation::JoinLines => text.replace('\n', ""),
        }
    }
}

fn mutation() -> impl Strategy<Value = Mutation> {
    prop_oneof![
        any::<usize>().prop_map(Mutation::Truncate),
        any::<usize>().prop_map(Mutation::DropLine),
        any::<usize>().prop_map(Mutation::DuplicateLine),
        (any::<usize>(), ".{0,60}").prop_map(|(at, text)| Mutation::ReplaceLine(at, text)),
        (
            hostile_number(),
            hostile_number(),
            hostile_number(),
            hostile_number()
        )
            .prop_map(|(a, b, c, d)| Mutation::LyingCounts(
                a.to_string(),
                b.to_string(),
                c.to_string(),
                d.to_string()
            )),
        Just(Mutation::Crlf),
        (
            any::<usize>(),
            proptest::collection::vec(any::<u8>(), 1..16)
        )
            .prop_map(|(at, bytes)| Mutation::Splice(at, bytes)),
        Just(Mutation::JoinLines),
    ]
}

fn with_lines(text: &str, edit: impl Fn(&mut Vec<String>)) -> String {
    let mut lines: Vec<String> = text.lines().map(ToString::to_string).collect();
    edit(&mut lines);
    let mut joined = lines.join("\n");
    if text.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// A byte offset inside `text` that is also a character boundary, so a mutation
/// cuts between characters rather than through one.
fn boundary(text: &str, at: usize) -> usize {
    if text.is_empty() {
        return 0;
    }
    let wanted = at % (text.len() + 1);
    (0..=wanted)
        .rev()
        .find(|candidate| text.is_char_boundary(*candidate))
        .unwrap_or(0)
}

/// Diffs git wrote, of a repository built to carry the shapes that hurt. They
/// are captured once and the mutations work on copies.
fn corpus() -> &'static [String] {
    static CORPUS: OnceLock<Vec<String>> = OnceLock::new();
    CORPUS.get_or_init(capture)
}

fn capture() -> Vec<String> {
    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.write("src/gone.ts", "export const gone = 1;\n");
    repo.write("src/renamed.ts", "export const kept = 1;\n");
    repo.write("script.sh", "#!/bin/sh\necho hello\n");
    repo.commit("the state the change starts from");
    let before = repo.head();

    repo.write(
        "src/parser.ts",
        "export const parse = (a: string) => a.split(\",\");\nexport const extra = 2;\n",
    );
    repo.remove("src/gone.ts");
    repo.git(&["mv", "src/renamed.ts", "src/moved.ts"]);
    repo.write(
        "src/crlf.ts",
        "export const a = 1;\r\nexport const b = 2;\r\n",
    );
    repo.write("src/no-newline.ts", "export const end = 1;");
    repo.write("a file with spaces.ts", "export const spaced = 1;\n");
    std::fs::write(repo.root().join("blob.bin"), [0_u8, 1, 2, 3, 255, 254, 0])
        .expect("a binary file should be writable");
    std::fs::write(repo.root().join("latin.txt"), [0xE9_u8, b'n', b'\n'])
        .expect("a latin-1 file should be writable");
    // A symlink is a file unix has and Windows has not: git there leaves
    // `core.symlinks` off and writes a plain file holding the target's path, so
    // the corpus carries the shape where the platform can hold one.
    #[cfg(unix)]
    std::os::unix::fs::symlink("src/parser.ts", repo.root().join("link.ts"))
        .expect("a symlink should be creatable");
    repo.git(&["update-index", "--chmod=+x", "script.sh"]);
    repo.stage_all();

    let mut corpus = vec![
        capture_one(
            &repo,
            &["diff", "--no-color", "--find-renames", "--cached", &before],
        ),
        capture_one(&repo, &["diff", "--no-color", "--find-renames", &before]),
        capture_one(
            &repo,
            &["diff", "--no-color", "--cached", "--unified=0", &before],
        ),
        capture_one(&repo, &["show", "--no-color", &before]),
    ];
    corpus.retain(|diff| !diff.trim().is_empty());
    assert!(
        corpus.len() >= 3,
        "the corpus needs real diffs to damage, and git produced {}",
        corpus.len()
    );
    corpus
}

/// git's answer as the seam hands it over: lossy text, so a byte that is not
/// utf-8 arrives as the replacement character rather than stopping the capture.
fn capture_one(repo: &Repo, arguments: &[&str]) -> String {
    let run = repo.try_git(arguments);
    assert_eq!(
        run.code,
        0,
        "git {} failed: {}",
        arguments.join(" "),
        run.stderr
    );
    run.stdout
}
