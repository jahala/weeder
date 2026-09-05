//! X1 — a secret-looking string was added.
//!
//! The fire fixture assigns one credential per line: every prefix an issuer
//! stamps, a private key block, a signed token, and a value that is secret by
//! its name and its disorder rather than by any prefix at all. The lines weed
//! must report are read off the fixture here by the shapes this file planted.
//!
//! The neighbour is the three ways a line looks like this and is not: a short
//! value under a key name, a placeholder waiting to be filled in, and the
//! digests a lockfile is made of.

mod common;

use common::{fixture, fixture_file};

/// The languages the rule is proven in, and the file the credentials land in.
const LANGUAGES: [(&str, &str); 4] = [
    ("ts", "src/config.ts"),
    ("py", "src/config.py"),
    ("rs", "src/config.rs"),
    ("go", "config.go"),
];

/// What the fire fixture planted, each of them once and each on its own line.
const SHAPES: [&str; 11] = [
    "AKIA",
    "ghp_",
    "github_pat_",
    "sk-",
    "xoxb-",
    "AIza",
    "glpat-",
    "npm_",
    "eyJ",
    "-----BEGIN",
    "9f3Kx2Qv",
];

/// The lockfile the neighbour carries, and the file the honest values live in.
const LOCKFILE: &str = "package-lock.json";

#[test]
fn x1_fires_at_block_level_on_every_prefix_and_on_a_disordered_value() {
    for (name, path) in LANGUAGES {
        let repo = fixture("X1", name, "fire");
        let run = repo.weed(&["check"]);

        assert_eq!(run.code, 2, "{name}: a credential blocks\n{}", run.stderr);
        let findings = run.findings();
        let planted = planted_lines(name, path);
        assert_eq!(
            planted.len(),
            SHAPES.len(),
            "{name}: the fixture plants each shape on a line of its own"
        );
        assert_eq!(
            findings
                .iter()
                .map(|finding| finding.line.expect("a credential finding names its line"))
                .collect::<Vec<u64>>(),
            planted.iter().map(|(line, _)| *line).collect::<Vec<u64>>(),
            "{name}: every planted line is reported, and only those"
        );
        for finding in &findings {
            assert_eq!(finding.rule, "X1", "{name}: the rule is X1");
            assert_eq!(finding.level, "error", "{name}: X1 blocks");
            assert_eq!(finding.path, path, "{name}: the file is named");
        }
        for ((_, value), finding) in planted.iter().zip(&findings) {
            assert!(
                !finding.message.contains(value.as_str()),
                "{name}: a finding never repeats the value it found: {}",
                finding.message
            );
        }
    }
}

#[test]
fn x1_stays_silent_on_a_short_value_a_placeholder_and_a_lockfile_digest() {
    for (name, path) in LANGUAGES {
        let repo = fixture("X1", name, "silent");

        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        for expected in [path, LOCKFILE] {
            assert!(
                changed.lines().any(|line| line == expected),
                "{name}: {expected} must be in the diff, or the silence proves nothing"
            );
        }
        let lock = fixture_file("X1", name, "silent/after", LOCKFILE);
        assert!(
            lock.split(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
                .any(|token| token.len() > 40),
            "{name}: the lockfile has to carry a digest, or it is not a neighbour"
        );
        let honest = fixture_file("X1", name, "silent/after", path);
        assert!(
            honest.contains("<your-key-here>"),
            "{name}: the neighbour has to carry a placeholder"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{name}: a setting, a slot to fill in and a digest are not credentials"
        );
        assert_eq!(run.code, 0, "{name}: nothing found, nothing blocked");
    }
}

/// Where each shape was planted, with the value the line assigns, counted from
/// one and in the order the file writes them.
fn planted_lines(lang: &str, path: &str) -> Vec<(u64, String)> {
    let source = fixture_file("X1", lang, "fire/after", path);
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| SHAPES.iter().any(|shape| line.contains(shape)))
        .map(|(index, line)| {
            (
                index as u64 + 1,
                quoted(line)
                    .unwrap_or_else(|| panic!("a planted line assigns a value: {line}"))
                    .to_string(),
            )
        })
        .collect()
}

/// What a line holds between its first pair of double quotes.
fn quoted(line: &str) -> Option<&str> {
    let opened = line.find('"')? + 1;
    let closed = line[opened..].find('"')?;
    Some(&line[opened..opened + closed])
}
