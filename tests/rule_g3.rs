//! G3, a control character was added to a text source file.
//!
//! One NUL byte makes git take a whole source file for binary, and from then on
//! every change to it reads `Binary files differ` with no line a reviewer can
//! see (#22). The other control characters print as nothing or move the cursor,
//! so the line a reviewer reads is not the line the file holds. Every case runs
//! the built binary on a real repository and reads the SARIF it wrote.

mod common;

use common::{fixture, fixture_file, Repo};

/// The languages the rule is proven in, and the file each fire fixture edits.
const LANGUAGES: [(&str, &str); 4] = [
    ("ts", "src/casting.ts"),
    ("py", "src/casting.py"),
    ("rs", "src/casting.rs"),
    ("go", "casting.go"),
];

/// The lines of the fire fixture's file that hold a NUL, counted from one,
/// read from the fixture rather than from the detector.
fn nul_lines(lang: &str, path: &str) -> Vec<u64> {
    fixture_file("G3", lang, "fire/after", path)
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains('\u{0}'))
        .map(|(index, _)| index as u64 + 1)
        .collect()
}

#[test]
fn g3_blocks_on_every_line_that_gained_a_nul_in_every_language() {
    for (lang, path) in LANGUAGES {
        let repo = fixture("G3", lang, "fire");
        assert!(
            repo.git(&["diff", "--cached", "--no-color"])
                .contains("Binary files"),
            "{lang}: git has to call the change binary, or the case proves nothing"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(run.code, 2, "{lang}: a NUL blocks\n{}", run.stderr);
        let findings = run.findings();
        let expected = nul_lines(lang, path);
        assert_eq!(expected.len(), 2, "{lang}: the fixture plants two NULs");
        let lines: Vec<u64> = findings
            .iter()
            .map(|finding| finding.line.expect("a G3 finding names its line"))
            .collect();
        assert_eq!(
            lines, expected,
            "{lang}: each line that gained a NUL is named, and no other"
        );
        for finding in &findings {
            assert_eq!(finding.rule, "G3", "{lang}: the rule is G3");
            assert_eq!(finding.level, "error", "{lang}: a NUL blocks");
            assert_eq!(finding.path, path, "{lang}: the finding names the file");
            assert!(
                finding.message.contains("U+0000"),
                "{lang}: the message names the code point: {}",
                finding.message
            );
        }
    }
}

#[test]
fn g3_warns_on_every_other_control_character_and_names_it() {
    let repo = Repo::init();
    repo.write("src/report.ts", "export const title = \"report\";\n");
    repo.commit("the state the change starts from");
    repo.write(
        "src/report.ts",
        "export const title = \"report\";\n\
         export const red = \"\u{1b}[31m\";\n\
         export const page = \"\u{c}\";\n\
         export const start = \"\u{1}\";\n\
         export const rubout = \"\u{7f}\";\n",
    );
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.code, 0,
        "a control character other than NUL warns, and a warning does not block\n{}",
        run.stderr
    );
    let findings = run.findings();
    for finding in &findings {
        assert_eq!(finding.rule, "G3", "the rule is G3");
        assert_eq!(finding.level, "warning", "anything but NUL warns");
    }
    let lines: Vec<Option<u64>> = findings.iter().map(|finding| finding.line).collect();
    assert_eq!(
        lines,
        vec![Some(2), Some(3), Some(4), Some(5)],
        "each line that gained a control character is named once"
    );
    for (finding, point) in findings
        .iter()
        .zip(["U+001B", "U+000C", "U+0001", "U+007F"])
    {
        assert!(
            finding.message.contains(point),
            "the message names {point}: {}",
            finding.message
        );
    }
}

#[test]
fn g3_stays_silent_on_a_tab_a_crlf_an_escape_written_as_text_a_byte_already_there_and_a_blob() {
    for (lang, _) in LANGUAGES {
        let repo = fixture("G3", lang, "silent");
        // The neighbours are in the bytes the change staged, or their silence
        // proves nothing: a tab and a carriage return on added lines, and a
        // NUL the base already held.
        let staged = repo.git(&["diff", "--cached", "--text", "--no-color"]);
        let added: Vec<&str> = staged
            .lines()
            .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
            .collect();
        for (byte, what) in [('\t', "a tab"), ('\r', "a carriage return")] {
            assert!(
                added.iter().any(|line| line.contains(byte)),
                "{lang}: the change adds {what}"
            );
        }
        assert!(
            staged
                .lines()
                .any(|line| line.starts_with(' ') && line.contains('\u{0}')),
            "{lang}: the change keeps a NUL the base held"
        );
        let run = repo.weeder(&["check"]);
        let g3: Vec<_> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule == "G3")
            .collect();
        assert_eq!(
            g3,
            Vec::new(),
            "{lang}: nothing here is G3's\n{}",
            run.stderr
        );
        assert!(
            run.findings().iter().any(|finding| finding.rule == "G2"),
            "{lang}: the added blob is G2's to report, so the silence above is G3 declining it"
        );
    }
}

#[test]
fn a_byte_the_line_already_held_is_not_reported_again_when_the_line_is_edited() {
    let repo = Repo::init();
    repo.write("src/legacy.ts", "export const retries = 2; // \u{0}\n");
    repo.commit("the state the change starts from");
    repo.write("src/legacy.ts", "export const retries = 3; // \u{0}\n");
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "the NUL was there before the change, and the change only moved a digit beside it"
    );
    assert_eq!(run.code, 0, "nothing found, nothing blocked");
}

#[test]
fn a_second_nul_in_a_file_that_held_one_is_reported() {
    let repo = Repo::init();
    repo.write("src/legacy.ts", "// \u{0}\nexport const retries = 2;\n");
    repo.commit("the state the change starts from");
    repo.write(
        "src/legacy.ts",
        "// \u{0}\nexport const retries = 2; // \u{0}\n",
    );
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    let lines: Vec<Option<u64>> = run.findings().iter().map(|finding| finding.line).collect();
    assert_eq!(
        lines,
        vec![Some(2)],
        "the new NUL is named on its line, and the old one is not\n{}",
        run.stderr
    );
    assert_eq!(run.code, 2, "a new NUL blocks");
}

#[test]
fn a_file_git_counted_as_text_at_the_base_is_judged_whatever_its_name() {
    let repo = Repo::init();
    repo.write("notes/plan.txt", "first step\nsecond step\n");
    repo.commit("the state the change starts from");
    repo.write("notes/plan.txt", "first step\nsecond \u{0} step\n");
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    let findings = run.findings();
    assert_eq!(
        findings
            .iter()
            .map(|finding| (finding.rule.as_str(), finding.line))
            .collect::<Vec<_>>(),
        vec![("G3", Some(2))],
        "a file that was text before the change is text that gained a NUL, and its line is named\n{}",
        run.stderr
    );
    assert_eq!(run.code, 2, "a NUL blocks");
}

#[test]
fn an_added_file_of_no_language_weeder_reads_is_left_to_g2() {
    let repo = Repo::init();
    repo.write("src/index.ts", "export const ready = true;\n");
    repo.commit("the state the change starts from");
    repo.write("notes/plan.txt", "first step\nsecond \u{0} step\n");
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    let rules: Vec<String> = run
        .findings()
        .into_iter()
        .map(|finding| finding.rule)
        .collect();
    assert_eq!(
        rules,
        vec!["G2"],
        "a new file nothing says is text is the blob G2 reports\n{}",
        run.stderr
    );
}

#[test]
fn an_added_source_file_that_carries_a_control_character_is_judged_too() {
    let repo = Repo::init();
    repo.write("src/index.ts", "export const ready = true;\n");
    repo.commit("the state the change starts from");
    repo.write("src/colour.ts", "export const red = \"\u{1b}[31m\";\n");
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    let findings = run.findings();
    assert_eq!(
        findings
            .iter()
            .map(|finding| (finding.rule.as_str(), finding.level.as_str(), finding.line))
            .collect::<Vec<_>>(),
        vec![("G3", "warning", Some(1))],
        "a new source file is a text source file from its first line\n{}",
        run.stderr
    );
}
