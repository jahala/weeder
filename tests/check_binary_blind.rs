//! git shows weeder every changed line of a source file, whatever the
//! repository says about diffing it.
//!
//! git decides a file is binary when an attribute says `-diff` or when its bytes
//! hold a NUL, and then writes `Binary files differ` in place of every hunk. A
//! judge that takes that at its word can be blinded by one line in
//! `.git/info/attributes`, which no diff ever shows, by a committed
//! `.gitattributes`, or by one NUL byte in a comment. So weeder asks for the text
//! of every change and reads a file whose language it knows as the text it is.
//!
//! Each blocking shape, a stub, a secret, a skip and a conflict marker, is taken
//! from its own fire fixture in every language, hidden each of the three ways,
//! and must still block. A genuinely binary file is the other half: it stays the
//! blob G2 reports, and no rule reads its bytes as lines.

mod common;

use std::collections::BTreeSet;

use common::{fixture, Repo};

const LANGUAGES: [&str; 4] = ["ts", "py", "rs", "go"];

/// The rules whose shapes a hidden hunk would hide, each with what it is.
const RULES: [(&str, &str); 4] = [
    ("S1", "a stub"),
    ("X1", "a secret"),
    ("T3", "a skip"),
    ("G1", "a conflict marker"),
];

/// The three ways a repository can tell git to show no lines.
#[derive(Debug, Clone, Copy)]
enum Blind {
    /// `* -diff` in `.git/info/attributes`, which lives outside every tree.
    InfoAttributes,
    /// A `.gitattributes` the base commit carries, marking the language `-diff`.
    CommittedAttributes,
    /// One NUL byte, in a comment, in every changed file of the language.
    NulByte,
}

const BLINDS: [Blind; 3] = [
    Blind::InfoAttributes,
    Blind::CommittedAttributes,
    Blind::NulByte,
];

fn extension(lang: &str) -> &'static str {
    match lang {
        "ts" => "ts",
        "py" => "py",
        "rs" => "rs",
        "go" => "go",
        other => panic!("no extension for {other}"),
    }
}

/// A comment line in the language, carrying one NUL byte.
fn nul_comment(lang: &str) -> &'static str {
    match lang {
        "py" => "# \u{0}\n",
        _ => "// \u{0}\n",
    }
}

/// The staged paths of the change, as git names them.
fn changed(repo: &Repo) -> Vec<String> {
    repo.git(&["diff", "--cached", "--name-only", "--no-renames"])
        .lines()
        .map(ToString::to_string)
        .collect()
}

fn hide(repo: &Repo, lang: &str, blind: Blind) {
    let ext = extension(lang);
    match blind {
        Blind::InfoAttributes => {
            let info = repo.root().join(".git/info");
            std::fs::create_dir_all(&info).expect("the info directory");
            std::fs::write(info.join("attributes"), "* -diff\n").expect("the attributes file");
        }
        Blind::CommittedAttributes => {
            repo.write(".gitattributes", &format!("*.{ext} -diff\n"));
            repo.git(&["add", ".gitattributes"]);
            // Only the attributes land; the change under judgement stays staged.
            repo.git(&["commit", "-m", "diff settings", "--", ".gitattributes"]);
        }
        Blind::NulByte => {
            for path in changed(repo) {
                if !path.ends_with(&format!(".{ext}")) {
                    continue;
                }
                let file = repo.root().join(&path);
                let Ok(text) = std::fs::read_to_string(&file) else {
                    continue;
                };
                repo.write(&path, &format!("{}{text}", nul_comment(lang)));
            }
            repo.stage_all();
        }
    }
}

/// The rules a run reported, each once.
fn rule_set(run: &common::Run) -> BTreeSet<String> {
    run.findings()
        .into_iter()
        .map(|finding| finding.rule)
        .collect()
}

/// Whether git itself now calls the change binary, so the test proves the
/// concealment took before it asks weeder anything.
fn git_is_blind(repo: &Repo) -> bool {
    repo.git(&["diff", "--cached", "--no-color"])
        .contains("Binary files")
}

#[test]
fn each_blocking_shape_still_blocks_when_git_is_told_to_show_no_lines() {
    let mut misses = Vec::new();
    for (rule, shape) in RULES {
        for lang in LANGUAGES {
            let seen =
                rule_set(&fixture(rule, lang, "fire").weeder(&["check", "--format", "sarif"]));
            for blind in BLINDS {
                let repo = fixture(rule, lang, "fire");
                hide(&repo, lang, blind);
                assert!(
                    git_is_blind(&repo),
                    "{rule} {lang} {blind:?}: git has to call the change binary, or the test proves nothing"
                );

                let run = repo.weeder(&["check", "--format", "sarif"]);
                let blocked = run
                    .findings()
                    .iter()
                    .any(|finding| finding.rule == rule && finding.level == "error");
                // What git was told to hide is judged as if it had been shown:
                // the same rules, and G2 besides where a NUL arrived in a new
                // file, because nobody can review that file on a page either.
                let mut hidden = rule_set(&run);
                if matches!(blind, Blind::NulByte) && !seen.contains("G2") {
                    hidden.remove("G2");
                }
                if run.code != 2 || !blocked || hidden != seen {
                    misses.push(format!(
                        "{rule} ({shape}) in {lang} under {blind:?}: exit {}, {hidden:?} where the plain change gives {seen:?}{}",
                        run.code, run.stderr
                    ));
                }
            }
        }
    }
    assert!(
        misses.is_empty(),
        "{} blocking shapes went unseen:\n{}",
        misses.len(),
        misses.join("\n")
    );
}

/// A PNG header, NULs, and bytes that spell a conflict marker and a key id on
/// lines of their own: a blob whose bytes happen to hold the shapes, which no
/// person wrote and nobody can review.
fn blob() -> Vec<u8> {
    let mut bytes = vec![
        0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', 0, 0, 0, 0x0d,
    ];
    bytes.extend_from_slice(b"\n<<<<<<< HEAD\n");
    bytes.extend_from_slice(b"AKIA");
    bytes.extend_from_slice(b"IOSFODNN7EXAMPLQ\n");
    bytes.extend_from_slice(&[0xff, 0xfe, 0, 0x80, 0x81, b'\n', 0]);
    bytes
}

#[test]
fn a_genuinely_binary_file_is_a_blob_g2_reports_and_no_rule_reads_as_lines() {
    let repo = Repo::init();
    repo.write("src/index.ts", "export const ready = true;\n");
    repo.commit("the state the change starts from");
    std::fs::create_dir_all(repo.root().join("assets")).expect("the assets directory");
    std::fs::write(repo.root().join("assets/mark.png"), blob()).expect("the blob");
    repo.stage_all();

    let run = repo.weeder(&["check", "--format", "sarif"]);

    let rules: Vec<String> = run
        .findings()
        .into_iter()
        .map(|finding| finding.rule)
        .collect();
    assert_eq!(
        rules,
        vec!["G2".to_string()],
        "an added blob is G2's to report, and its bytes are not lines: {}",
        run.output()
    );
    assert_eq!(run.code, 0, "G2 warns: {}", run.stderr);
}

#[test]
fn a_changed_binary_file_says_nothing() {
    let repo = Repo::init();
    std::fs::create_dir_all(repo.root().join("assets")).expect("the assets directory");
    std::fs::write(repo.root().join("assets/mark.png"), b"\x89PNG\0\0").expect("the blob");
    repo.commit("the mark");
    std::fs::write(repo.root().join("assets/mark.png"), blob()).expect("the blob");
    repo.stage_all();

    let run = repo.weeder(&["check", "--format", "sarif"]);

    assert_eq!(
        run.findings(),
        Vec::new(),
        "a tracked blob that changed was said yes to before: {}",
        run.output()
    );
    assert_eq!(run.code, 0);
}
