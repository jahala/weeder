//! X1, a secret-looking string was added.
//!
//! The fire fixture assigns one credential per line: every prefix an issuer
//! stamps, a private key block, a signed token, and a value that is secret by
//! its name and its disorder rather than by any prefix at all. The lines weed
//! must report are read off the fixture here by the shapes this file planted.
//!
//! The neighbour is the three ways a line looks like this and is not: a short
//! value under a key name, a placeholder waiting to be filled in, and the
//! digests a lockfile is made of.
//!
//! The two paths reach different files. A stamped token is a credential
//! wherever it lands, a note, a page, a log, so the prefix path reads every
//! file. Reading a name and a value apart takes a language weed knows the
//! grammar of, and outside one, `name = value` is a css class, an attribute or
//! a sentence with a colon in it, so the second path reads source alone.

mod common;

use common::{fixture, fixture_file, Finding};

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

        // The lockfile has to be in the diff for the digest to be judged at
        // all, and a changed lockfile is D1's business (rules-prod). What this
        // case is about is that none of the three is read as a credential.
        let run = repo.weed(&["check"]);
        let credentials: Vec<Finding> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule == "X1")
            .collect();
        assert_eq!(
            credentials,
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

/// The prose fixture's languages folder: the files weed has no grammar for.
const PROSE: &str = "prose";

/// What the prose fire fixture plants, one stamped token per file, in the order
/// weed reports them.
const PLANTED: [(&str, &str); 3] = [
    ("deploy/notes.txt", "sk-"),
    ("docs/incident.md", "ghp_"),
    ("web/index.html", "AKIA"),
];

/// The files the prose neighbour writes a name-and-value shape into.
const NEIGHBOURS: [&str; 2] = ["docs/rotating.md", "web/index.html"];

/// The disordered value the language fixtures assign to a key-like name: the
/// one shape that is a credential by its name and its disorder alone.
const DISORDERED: &str = "9f3Kx2Qv";

#[test]
fn x1_fires_on_a_stamped_token_in_a_file_weed_has_no_grammar_for() {
    let repo = fixture("X1", PROSE, "fire");
    let run = repo.weed(&["check"]);

    assert_eq!(
        run.code, 2,
        "a credential blocks wherever it lands\n{}",
        run.stderr
    );
    let findings: Vec<Finding> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == "X1")
        .collect();
    assert_eq!(
        findings.len(),
        PLANTED.len(),
        "one finding per planted token: {findings:#?}"
    );
    for ((path, stamp), finding) in PLANTED.iter().zip(&findings) {
        assert_eq!(finding.path, *path, "the file is named");
        assert_eq!(finding.level, "error", "{path}: X1 blocks");
        let planted = fixture_file("X1", PROSE, "fire/after", path)
            .lines()
            .position(|line| line.contains(stamp))
            .map(|index| index as u64 + 1)
            .unwrap_or_else(|| panic!("{path} plants a token"));
        assert_eq!(finding.line, Some(planted), "{path}: the planted line");
    }
}

#[test]
fn x1_stays_silent_on_a_markup_attribute_and_on_prose_about_credentials() {
    let repo = fixture("X1", PROSE, "silent");

    let changed = repo.git(&["diff", "HEAD", "--name-only"]);
    for path in NEIGHBOURS {
        assert!(
            changed.lines().any(|line| line == path),
            "{path} must be in the diff, or the silence proves nothing"
        );
        let after = fixture_file("X1", PROSE, "silent/after", path);
        assert!(
            after.lines().any(looks_assigned),
            "{path} has to carry a name-and-value shape a key-like name answers for, or it is not a neighbour"
        );
    }

    let run = repo.weed(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "a class attribute and a sentence with a colon in it are not assignments"
    );
    assert_eq!(run.code, 0, "nothing found, nothing blocked");
}

#[test]
fn the_name_and_disorder_path_still_reads_source_in_every_language() {
    for (name, path) in LANGUAGES {
        let repo = fixture("X1", name, "fire");
        let planted = fixture_file("X1", name, "fire/after", path)
            .lines()
            .position(|line| line.contains(DISORDERED))
            .map(|index| index as u64 + 1)
            .unwrap_or_else(|| panic!("{name}: the fixture assigns a disordered value"));

        let run = repo.weed(&["check"]);
        let finding = run
            .findings()
            .into_iter()
            .find(|finding| finding.rule == "X1" && finding.line == Some(planted))
            .unwrap_or_else(|| {
                panic!("{name}: the disordered value on line {planted} is reported")
            });
        assert_eq!(finding.level, "error", "{name}: X1 blocks");
        assert!(
            !finding.message.contains(DISORDERED),
            "{name}: a finding never repeats the value it found: {}",
            finding.message
        );
    }
}

/// Whether a line writes a name, an assignment and a value long and disordered
/// enough to read as a credential. This is the shape a neighbour has to carry
/// for its silence to mean anything, worked out here rather than asked of the
/// rule under test.
fn looks_assigned(line: &str) -> bool {
    line.match_indices(['=', ':']).any(|(at, _)| {
        let named = line[..at]
            .split(|character: char| {
                !(character.is_alphanumeric() || character == '_' || character == '-')
            })
            .any(holds_key_word);
        let Some(value) = quoted_by_any(&line[at + 1..]) else {
            return false;
        };
        named && value.chars().count() >= 20 && entropy(value) > 4.0
    })
}

/// What a value is written between, whichever quotation the format uses.
fn quoted_by_any(tail: &str) -> Option<&str> {
    let trimmed = tail.trim_start();
    let opener = trimmed.chars().next()?;
    if !matches!(opener, '"' | '\'' | '`') {
        return None;
    }
    let opened = opener.len_utf8();
    let end = trimmed[opened..].find(opener)?;
    Some(&trimmed[opened..opened + end])
}

/// Whether a name is written from a word that names a credential.
fn holds_key_word(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    ["key", "secret", "token", "password", "credential"]
        .iter()
        .any(|word| lowered.contains(word))
}

/// How many bits of surprise each character of a value carries.
fn entropy(value: &str) -> f64 {
    let characters: Vec<char> = value.chars().collect();
    let total = characters.len() as f64;
    if total == 0.0 {
        return 0.0;
    }
    let mut counts: Vec<(char, usize)> = Vec::new();
    for character in characters {
        match counts.iter_mut().find(|(seen, _)| *seen == character) {
            Some((_, count)) => *count += 1,
            None => counts.push((character, 1)),
        }
    }
    -counts
        .iter()
        .map(|(_, count)| {
            let share = *count as f64 / total;
            share * share.log2()
        })
        .sum::<f64>()
}
