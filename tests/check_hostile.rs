//! The binary, on repositories built to hurt it.
//!
//! Everything here is a real git repository holding something an agent can and
//! does produce: a symlink, a submodule, a merge with two parents, a commit that
//! changes nothing, a file too big to read twice, CRLF line ends, bytes that are
//! not text, bytes that are not utf-8, and paths with spaces and quotes in them.
//!
//! Every run has to leave with a code the contract names, 0 clean, 2 blocked,
//! 3 could not run, and a 3 has to say why on one line. A status of 101 is a
//! Rust panic, and a panic is a gate that stopped judging without saying so.
//!
//! Where weeder can judge, this file also says what it must find. A test that only
//! watched for exit 101 would go on passing after weeder started refusing every
//! one of these repositories, which is a different way of not judging them.

mod common;

use std::path::Path;

use common::{ours, separator, theirs, Repo, Run, BRANCH};

/// The codes weeder's contract names. Anything else, above all 101, which is what
/// a panicking Rust binary leaves with, is a run nobody can read.
const ALLOWED: [i32; 3] = [0, 2, 3];

/// The one thing every run in this file has to do, whatever it made of the
/// repository: leave with a code that means something, and say why if it could
/// not run at all.
#[track_caller]
fn survives(run: &Run, what: &str) {
    assert!(
        ALLOWED.contains(&run.code),
        "{what}: weeder left with {}, which its contract does not name{}\n{}\n{}",
        run.code,
        if run.code == 101 {
            ", 101 is a panic"
        } else {
            ""
        },
        run.stdout,
        run.stderr
    );
    if run.code == 3 {
        let lines = run.stderr_lines();
        assert_eq!(
            lines.len(),
            1,
            "{what}: a run that could not run says why on one line: {}",
            run.stderr
        );
        assert!(
            !lines[0].trim().is_empty(),
            "{what}: the one line has to say something"
        );
    }
}

/// A run that reached a judgement wrote one: the SARIF a machine reads has to
/// parse, whatever was in the repository. Without this a repository weeder had
/// quietly started refusing would still pass every test in this file.
#[track_caller]
fn judged_in_sarif(run: &Run, what: &str) {
    survives(run, what);
    assert_ne!(run.code, 3, "{what}: weeder could not run: {}", run.stderr);
    let log = run.log();
    assert!(
        log["runs"][0]["results"].is_array(),
        "{what}: the log carries a results array: {}",
        run.stdout
    );
    assert_eq!(
        log["runs"][0]["invocations"][0]["executionSuccessful"],
        serde_json::Value::Bool(true),
        "{what}: the run says it reached a judgement"
    );
}

/// The views a caller has of a repository. Every one of them reads the same
/// tree through a different diff, so every one of them meets the same shapes.
fn every_view(repo: &Repo, base: &str, what: &str) {
    for arguments in [
        vec!["check", "--format", "sarif"],
        vec!["check", "--format", "table"],
        vec!["check", "--format", "sarif", "--staged"],
        vec!["check", "--format", "sarif", "--strict"],
        vec!["check", "--format", "sarif", "--base", base],
        vec!["check", "--format", "table", "--base", base],
    ] {
        let run = repo.weeder(&arguments);
        let what = format!("{what} judged by `weeder {}`", arguments.join(" "));
        if arguments.contains(&"sarif") {
            judged_in_sarif(&run, &what);
        } else {
            survives(&run, &what);
        }
    }
}

/// A file git could not merge, with both sides still in it. Every repository
/// here carries one, so weeder has something to find and the test can tell
/// judging from refusing.
fn conflicted(language: &str) -> String {
    format!(
        "export function parse(input: {language}) {{\n{opener}\n  return 1;\n{separator}\n  return 2;\n{closer}\n}}\n",
        opener = ours("HEAD"),
        separator = separator(),
        closer = theirs(BRANCH),
    )
}

/// The line the opener sits on in [`conflicted`].
const CONFLICT_LINE: u64 = 2;

/// The diff git itself writes for what is staged. A test reads it to say the
/// repository really holds the shape the test is named after, rather than
/// trusting that `git add` did what it looked like it did.
fn staged_diff(repo: &Repo, base: &str) -> String {
    repo.git(&["diff", "--no-color", "--find-renames", "--cached", base])
}

/// A symlink in a work tree is a shape unix has and Windows has not: git there
/// leaves `core.symlinks` off and writes a plain file holding the target's path,
/// so there is no link for weeder to walk past.
#[cfg(unix)]
#[test]
fn a_symlink_to_a_file_and_to_a_directory_are_judged() {
    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.commit("the state the change starts from");
    let base = repo.head();

    repo.write("src/conflicted.ts", &conflicted("string"));
    symlink("src/parser.ts", repo.root().join("to-a-file.ts"));
    symlink("src", repo.root().join("to-a-directory"));
    symlink("nowhere.ts", repo.root().join("to-nothing.ts"));
    repo.stage_all();
    assert!(
        staged_diff(&repo, &base).contains("new file mode 120000"),
        "the repository really holds symlinks"
    );

    every_view(&repo, &base, "a repository with symlinks");
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(run.code, 2, "the conflict is still found: {}", run.stderr);
    assert!(
        run.paths().contains(&"src/conflicted.ts".to_string()),
        "a symlink beside a file does not stop weeder reading the file: {:?}",
        run.paths()
    );
}

#[test]
fn a_submodule_is_judged() {
    let inner = Repo::init();
    inner.write("src/inner.ts", "export const inner = 1;\n");
    inner.commit("the submodule has something in it");

    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.commit("the state the change starts from");
    let base = repo.head();

    repo.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        &inner.root().display().to_string(),
        "vendor/inner",
    ]);
    repo.write("src/conflicted.ts", &conflicted("string"));
    repo.stage_all();
    assert!(
        staged_diff(&repo, &base).contains("new file mode 160000"),
        "the repository really holds a submodule"
    );

    every_view(&repo, &base, "a repository with a submodule");
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(run.code, 2, "the conflict is still found: {}", run.stderr);
    assert!(
        run.paths().contains(&"src/conflicted.ts".to_string()),
        "a gitlink in the diff does not stop weeder reading the rest: {:?}",
        run.paths()
    );
}

#[test]
fn a_merge_commit_with_two_parents_is_judged_against_a_base() {
    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.commit("the state the change starts from");
    let base = repo.head();

    repo.git(&["checkout", "-b", "side"]);
    repo.write("src/side.ts", "export const side = 1;\n");
    repo.commit("the side branch adds a file");

    repo.git(&["checkout", "main"]);
    repo.write("src/main.ts", "export const main = 1;\n");
    repo.commit("main adds one too");
    repo.git(&[
        "merge",
        "--no-ff",
        "-m",
        "the branches come back together",
        "side",
    ]);

    let merge = repo.head();
    assert_eq!(
        repo.git(&["rev-list", "--parents", "-n", "1", &merge])
            .split_whitespace()
            .count(),
        3,
        "the merge commit has two parents"
    );

    repo.write("src/conflicted.ts", &conflicted("string"));
    repo.stage_all();

    every_view(&repo, &base, "a repository with a merge commit");
    let run = repo.weeder(&["check", "--format", "sarif", "--base", &base]);
    assert_eq!(
        run.code, 2,
        "a range spanning a merge is still judged: {}",
        run.stderr
    );
    assert!(
        run.paths().contains(&"src/conflicted.ts".to_string()),
        "{:?}",
        run.paths()
    );
}

#[test]
fn a_commit_that_changed_nothing_is_judged() {
    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.commit("the state the change starts from");
    let base = repo.head();
    repo.git(&[
        "commit",
        "--allow-empty",
        "-m",
        "a commit that changed nothing",
    ]);

    every_view(&repo, &base, "a repository with an empty commit");
    let run = repo.weeder(&["check", "--format", "sarif", "--base", &base]);
    assert_eq!(
        run.code, 0,
        "a range that changed nothing is clean, not broken: {}",
        run.stderr
    );
    assert!(
        run.findings().is_empty(),
        "nothing changed, so nothing is found"
    );
}

#[test]
fn a_twenty_mebibyte_file_is_judged() {
    let repo = Repo::init();
    let base = repo.head();

    // Twenty mebibytes of ordinary source lines: no NUL byte, so git calls it
    // text and writes every line of it into the diff.
    let line = "export const value = 1; // a line that is long enough to be worth counting\n";
    let mut huge = String::with_capacity(21 * 1024 * 1024);
    while huge.len() < 20 * 1024 * 1024 {
        huge.push_str(line);
    }
    repo.write("src/huge.ts", &huge);
    repo.write("src/conflicted.ts", &conflicted("string"));
    repo.stage_all();
    let diff = staged_diff(&repo, &base);
    assert!(
        diff.len() > 20 * 1024 * 1024,
        "git writes every line of the big file into the diff, and wrote {} bytes",
        diff.len()
    );
    assert!(
        !diff.contains("Binary files"),
        "git reads the big file as text, so weeder has twenty mebibytes of lines to parse"
    );

    let run = repo.weeder(&["check", "--format", "sarif"]);
    survives(&run, "a repository with a twenty mebibyte file");
    assert_eq!(
        run.code, 2,
        "the conflict beside the big file is still found: {}",
        run.stderr
    );
    let base_run = repo.weeder(&["check", "--format", "table", "--base", &base]);
    survives(&base_run, "a twenty mebibyte file judged against a base");
}

#[test]
fn crlf_line_ends_are_judged_and_the_conflict_in_them_is_found() {
    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.commit("the state the change starts from");
    let base = repo.head();

    repo.write("src/crlf.ts", &conflicted("string").replace('\n', "\r\n"));
    repo.stage_all();

    every_view(&repo, &base, "a repository with CRLF line ends");
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(
        run.code, 2,
        "a conflict marker is a conflict marker whatever ends the line: {}",
        run.stderr
    );
    let found = run.findings();
    assert!(
        found
            .iter()
            .any(|finding| finding.path == "src/crlf.ts" && finding.line == Some(CONFLICT_LINE)),
        "the marker is reported on the line it is on: {found:?}"
    );
}

#[test]
fn a_binary_file_is_judged() {
    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.commit("the state the change starts from");
    let base = repo.head();

    // A NUL in the first few bytes is how git decides a file is not text.
    let mut bytes: Vec<u8> = vec![0, 1, 2, 3, 255, 254, 0, 128];
    bytes.extend((0..4096).map(|index| (index % 251) as u8));
    std::fs::write(repo.root().join("assets/blob.bin"), &bytes)
        .or_else(|_| {
            std::fs::create_dir_all(repo.root().join("assets"))?;
            std::fs::write(repo.root().join("assets/blob.bin"), &bytes)
        })
        .expect("a binary file should be writable");
    repo.write("src/conflicted.ts", &conflicted("string"));
    repo.stage_all();
    assert!(
        staged_diff(&repo, &base).contains("Binary files"),
        "git reads the file as binary, which is the shape this test is about"
    );

    every_view(&repo, &base, "a repository with a binary file");
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(run.code, 2, "the conflict is still found: {}", run.stderr);
    let blob: Vec<String> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.path == "assets/blob.bin")
        .map(|finding| finding.rule)
        .collect();
    assert_eq!(
        blob,
        vec!["G2".to_string()],
        "a binary file has no lines, so the only thing weeder says about it is that it arrived"
    );
}

#[test]
fn text_that_is_not_utf_eight_is_judged() {
    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.commit("the state the change starts from");
    let base = repo.head();

    // Latin-1: no NUL, so git calls it text and puts the bytes in the diff.
    let mut bytes = b"const caf".to_vec();
    bytes.extend_from_slice(&[0xE9, b'\n']);
    bytes.extend_from_slice("const rest = 1;\n".as_bytes());
    std::fs::write(repo.root().join("src/latin.ts"), &bytes)
        .expect("a latin-1 file should be writable");
    repo.write("src/conflicted.ts", &conflicted("string"));
    repo.stage_all();
    let diff = staged_diff(&repo, &base);
    assert!(
        !diff.contains("Binary files a/src/latin.ts"),
        "git reads the latin-1 file as text, so its bytes reach weeder"
    );

    every_view(&repo, &base, "a repository with text that is not utf-8");
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(
        run.code, 2,
        "a byte weeder cannot read does not stop it judging the rest: {}",
        run.stderr
    );
    assert!(
        run.paths().contains(&"src/conflicted.ts".to_string()),
        "the file beside it is still judged: {:?}",
        run.paths()
    );
}

/// Two of these names are unix's to spell: NTFS refuses a quote in a filename
/// outright, and a backslash there separates directories rather than being a
/// character in a name. The rest of the list is judged on both platforms by the
/// tests above.
#[cfg(unix)]
#[test]
fn paths_with_spaces_and_quotes_are_judged_and_named_as_they_are_spelled() {
    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.commit("the state the change starts from");
    let base = repo.head();

    let awkward = [
        "src/a file with spaces.ts",
        "src/a\"quoted\".ts",
        "src/a directory with spaces/parser.ts",
        "src/ünïcode.ts",
        "src/a'single'.ts",
        "src/back\\slash.ts",
    ];
    for path in awkward {
        repo.write(path, &conflicted("string"));
    }
    repo.stage_all();

    every_view(&repo, &base, "a repository with awkward paths");
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(run.code, 2, "the conflicts are found: {}", run.stderr);
    let reported = run.paths();
    for path in awkward {
        assert!(
            reported.contains(&path.to_string()),
            "weeder names {path} the way the repository spells it, and reported {reported:?}"
        );
    }
}

#[test]
fn a_rename_is_judged() {
    let repo = Repo::init();
    repo.write("src/parser.ts", &conflicted("string"));
    repo.write("src/kept.ts", "export const kept = 1;\n");
    repo.commit("the state the change starts from");
    let base = repo.head();

    repo.git(&["mv", "src/parser.ts", "src/renamed parser.ts"]);
    repo.git(&["mv", "src/kept.ts", "src/moved.ts"]);
    repo.stage_all();

    every_view(&repo, &base, "a repository with renames");
    let run = repo.weeder(&["check", "--format", "sarif"]);
    survives(&run, "a rename judged");
}

/// All of it in one repository, judged every way a caller can ask. The shapes
/// meet each other here, which is where a parser that handles each one alone
/// tends to come apart.
/// Two of the shapes in here are unix's alone: a path with a quote in it, which
/// NTFS refuses, and a symlink, which git for Windows writes as a plain file.
/// The tree is the one weeder has to survive whole, so it is built where every
/// shape in it exists rather than built short.
#[cfg(unix)]
#[test]
fn everything_at_once_is_judged() {
    let inner = Repo::init();
    inner.write("src/inner.ts", "export const inner = 1;\n");
    inner.commit("the submodule has something in it");

    let repo = Repo::init();
    repo.write("src/parser.ts", "export const parse = (a: string) => a;\n");
    repo.write("src/gone.ts", "export const gone = 1;\n");
    repo.write("src/renamed.ts", "export const kept = 1;\n");
    repo.commit("the state the change starts from");
    let base = repo.head();

    repo.git(&["checkout", "-b", "side"]);
    repo.write("src/side.ts", "export const side = 1;\n");
    repo.commit("the side branch adds a file");
    repo.git(&["checkout", "main"]);
    repo.git(&[
        "merge",
        "--no-ff",
        "-m",
        "the branches come back together",
        "side",
    ]);
    repo.git(&[
        "commit",
        "--allow-empty",
        "-m",
        "a commit that changed nothing",
    ]);

    repo.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        &inner.root().display().to_string(),
        "vendor/inner",
    ]);
    repo.remove("src/gone.ts");
    repo.git(&["mv", "src/renamed.ts", "src/moved file.ts"]);
    repo.write("src/crlf.ts", &conflicted("string").replace('\n', "\r\n"));
    repo.write("src/a\"quoted\".ts", &conflicted("string"));
    repo.write("src/no-newline.ts", "export const end = 1;");
    std::fs::write(repo.root().join("src/latin.ts"), [b'a', 0xE9, b'\n'])
        .expect("a latin-1 file should be writable");
    std::fs::write(repo.root().join("src/blob.bin"), [0_u8, 1, 2, 255, 0])
        .expect("a binary file should be writable");
    symlink("src/parser.ts", repo.root().join("src/link.ts"));
    symlink("src", repo.root().join("to-a-directory"));
    repo.stage_all();

    every_view(&repo, &base, "a repository carrying everything at once");
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(
        run.code, 2,
        "the conflicts in it are still found: {}",
        run.stderr
    );
    for path in ["src/crlf.ts", "src/a\"quoted\".ts"] {
        assert!(
            run.paths().contains(&path.to_string()),
            "{path} is judged among all the rest: {:?}",
            run.paths()
        );
    }
}

/// A directory that is not a repository at all. weeder cannot judge a diff that
/// does not exist, and says so on one line rather than reporting a clean tree.
#[test]
fn a_directory_that_is_not_a_repository_leaves_with_a_reason() {
    let directory = tempfile::tempdir().expect("a temp directory");
    let run = common::weeder_in(directory.path(), &["check", "--format", "sarif"]);
    survives(&run, "a directory that is not a repository");
    assert_eq!(run.code, 3, "no repository, no judgement: {}", run.stdout);
}

/// A symlink, which is a unix file: Windows makes one only with a privilege
/// nobody grants a test, and git for Windows would write a plain file anyway.
#[cfg(unix)]
fn symlink(target: &str, at: std::path::PathBuf) {
    if let Some(parent) = at.parent() {
        std::fs::create_dir_all(parent).expect("a directory for the symlink");
    }
    std::os::unix::fs::symlink(Path::new(target), &at).expect("a symlink should be creatable");
}
