//! S4, production code was added that only its tests call.
//!
//! An agent writes a function and a test for it, and never wires a caller. The
//! test passes, the change looks finished, and nothing the product runs ever
//! reaches the new code. R2 cannot see this: the test is a reference, so the
//! export is not dead by its count. S4 judges the change instead, and asks
//! where the calls to what it exported come from.
//!
//! The neighbours are the same export with a production caller beside the
//! test, one somebody outside the repository is meant to call, and one nothing
//! references at all, which is R2's to report and not this rule's.

mod common;

use common::{fixture, fixture_file, Repo};
use weeder::core::change::READABLE;

/// The production file each fire fixture adds an export to, and the name it
/// exports.
const SHAPES: [(&str, &str, &str); 4] = [
    ("ts", "src/text.ts", "slugify"),
    ("py", "src/text.py", "slugify"),
    ("rs", "src/text.rs", "slugify"),
    ("go", "text.go", "Slugify"),
];

/// The file each silent fixture wires the export in from, and the entry point
/// each `silent-entry` fixture adds it to.
const WIRED: [(&str, &str, &str); 4] = [
    ("ts", "src/app.ts", "src/index.ts"),
    ("py", "src/app.py", "src/text/__init__.py"),
    ("rs", "src/app.rs", "src/lib.rs"),
    ("go", "app.go", "cli/text.go"),
];

#[test]
fn s4_warns_on_an_added_export_only_its_tests_call_in_every_language() {
    for (lang, path, name) in SHAPES {
        let repo = fixture("S4", lang, "fire");
        let run = repo.weeder(&["check"]);

        assert_eq!(
            run.code, 0,
            "{lang}: code only tests call is a warning, and a warning does not block\n{}",
            run.stderr
        );
        let findings = run.findings();
        assert_eq!(
            findings.len(),
            1,
            "{lang}: one export, one finding: {findings:?}\n{}",
            run.stderr
        );
        let finding = &findings[0];
        assert_eq!(finding.rule, "S4", "{lang}: the rule is S4");
        assert_eq!(finding.level, "warning", "{lang}: S4 warns");
        assert_eq!(finding.path, path, "{lang}: the finding names the file");
        assert_eq!(
            finding.line,
            Some(declared_at(lang, path, name)),
            "{lang}: the finding points at the declaration"
        );
        assert!(
            finding.message.contains(&format!("`{name}`")),
            "{lang}: the message names the export: {}",
            finding.message
        );
    }
}

#[test]
fn s4_stays_silent_where_production_code_calls_the_export_too() {
    for (lang, caller, _) in WIRED {
        let repo = fixture("S4", lang, "silent");
        let changed = repo.git(&["diff", "--cached", "--name-only"]);
        assert!(
            changed.lines().any(|line| line == caller),
            "{lang}: {caller} must be in the diff, or the silence proves nothing"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{lang}: a production caller wires the export in\n{}",
            run.stderr
        );
        assert_eq!(run.code, 0, "{lang}: nothing found, nothing blocked");
    }
}

#[test]
fn s4_stays_silent_on_an_entry_point_somebody_outside_the_repository_calls() {
    for (lang, _, entry) in WIRED {
        let repo = fixture("S4", lang, "silent-entry");
        let changed = repo.git(&["diff", "--cached", "--name-only"]);
        assert!(
            changed.lines().any(|line| line == entry),
            "{lang}: {entry} must be in the diff, or the silence proves nothing"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{lang}: what an entry point exports is somebody else's to call\n{}",
            run.stderr
        );
    }
}

#[test]
fn s4_stays_silent_on_a_file_the_repository_says_another_program_loads_by_path() {
    // The fire fixture's change, with the file that gains the export named
    // under `[entrypoints] loaded`: a program outside the repository loads it
    // by path, so its exports are that program's to call, and tests reaching
    // them is no sign that nothing else does.
    for (lang, path, _) in SHAPES {
        let repo = Repo::init();
        for (file, text) in fixture_side(lang, "fire/before") {
            repo.write(&file, &text);
        }
        repo.write(
            "weeder.toml",
            &format!("[entrypoints]\nloaded = [\"{path}\"]\n"),
        );
        repo.commit("the state the change starts from");
        for (file, text) in fixture_side(lang, "fire/after") {
            repo.write(&file, &text);
        }
        repo.stage_all();

        let run = repo.weeder(&["check"]);
        let s4: Vec<_> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule == "S4")
            .collect();
        assert_eq!(
            s4,
            Vec::new(),
            "{lang}: a file loaded by path is an entry point\n{}",
            run.stderr
        );
    }
}

/// Every file of one side of a fixture, as paths relative to the side.
fn fixture_side(lang: &str, side: &str) -> Vec<(String, String)> {
    let root = common::fixture_root().join("S4").join(lang).join(side);
    let mut files = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("the fixture side should be readable") {
            let path = entry.expect("a fixture entry should read").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path
                    .strip_prefix(&root)
                    .expect("a fixture file sits under its side")
                    .to_string_lossy()
                    .replace('\\', "/");
                let text = std::fs::read_to_string(&path).expect("a fixture file should read");
                files.push((relative, text));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn a_production_reference_that_is_not_a_call_still_wires_the_export_in() {
    // Handed to `map` rather than called, the function is still on a path the
    // product runs, and the caller search sees no call to it there.
    let repo = typescript_repo();
    repo.write(
        "src/app.ts",
        "import { slugify, trim } from \"./text\";\n\nexport function title(input: string): string {\n  return trim(input);\n}\n\nexport function links(names: string[]): string[] {\n  return names.map(slugify);\n}\n",
    );
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "a function passed as a value is used\n{}",
        run.stderr
    );
}

#[test]
fn a_file_past_the_size_weeder_reads_as_code_is_not_read_for_references() {
    // Past the cap a file is weighed and G2 reports it, and nobody reads it as
    // code, the caller search included. Its bytes naming the export wire nothing
    // in, and reading them for the name is what would cost the worst case its
    // budget.
    let repo = typescript_repo();
    let reference = "import { slugify } from \"./text\";\n\nexport const links = (names: string[]) => names.map(slugify);\n";
    let row = "export const field = \"the record carries a field\";\n";
    let padding = row.repeat(READABLE as usize / row.len() + 1);
    repo.write("src/generated.ts", &format!("{reference}{padding}"));
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    let s4: Vec<_> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == "S4")
        .collect();
    assert_eq!(
        s4.len(),
        1,
        "only tests reach the export from a file weeder reads as code: {s4:?}\n{}",
        run.stderr
    );
    assert_eq!(s4[0].path, "src/text.ts");
}

#[test]
fn a_test_written_first_is_found_by_the_caller_search_though_the_change_leaves_it_alone() {
    // The suite landed a commit earlier, the way a change written test first
    // lands. The change that exports the function does not touch it, so only
    // the caller search, walking the whole tree, can see who calls it.
    let repo = Repo::init();
    write_all(&repo, &fixture_tree("fire/before"));
    repo.write(
        "src/text.test.ts",
        &fixture_file("S4", "ts", "fire/after", "src/text.test.ts"),
    );
    repo.commit("the test, before what it tests");
    repo.write(
        "src/text.ts",
        &fixture_file("S4", "ts", "fire/after", "src/text.ts"),
    );
    repo.stage_all();
    let changed = repo.git(&["diff", "--cached", "--name-only"]);
    assert_eq!(
        changed.trim(),
        "src/text.ts",
        "the change touches the export alone"
    );

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.findings()
            .iter()
            .map(|finding| finding.rule.as_str())
            .collect::<Vec<_>>(),
        vec!["S4"],
        "an unchanged suite calling it is still only a suite\n{}",
        run.stderr
    );
}

#[test]
fn a_production_caller_the_change_leaves_alone_still_wires_the_export_in() {
    let repo = Repo::init();
    write_all(&repo, &fixture_tree("fire/before"));
    repo.write(
        "src/app.ts",
        &fixture_file("S4", "ts", "silent/after", "src/app.ts"),
    );
    repo.commit("the caller, before what it calls");
    write_all(&repo, &fixture_tree("fire/after"));
    repo.write(
        "src/app.ts",
        &fixture_file("S4", "ts", "silent/after", "src/app.ts"),
    );
    repo.stage_all();
    let changed = repo.git(&["diff", "--cached", "--name-only"]);
    assert!(
        !changed.lines().any(|line| line == "src/app.ts"),
        "the change leaves the caller alone"
    );

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "the caller search finds the production call in a file the change never opened\n{}",
        run.stderr
    );
}

#[test]
fn an_export_nothing_references_is_left_to_r2() {
    let repo = typescript_repo();
    repo.remove("src/text.test.ts");
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "with no test calling it either, the export is dead code, and that is R2's\n{}",
        run.stderr
    );
}

#[test]
fn an_export_the_base_already_held_is_not_judged_again() {
    // The change edits the body of a function that was already there and
    // already only tested. What the change added is a line, not an export.
    let repo = Repo::init();
    write_all(&repo, &fixture_tree("fire/after"));
    repo.commit("the export and its test");
    repo.write(
        "src/text.ts",
        &fixture_file("S4", "ts", "fire/after", "src/text.ts")
            .replace("join(\"-\")", "join(\"_\")"),
    );
    repo.write(
        "src/text.test.ts",
        &fixture_file("S4", "ts", "fire/after", "src/text.test.ts").replace("a-b", "a_b"),
    );
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "only what the change exports is S4's to judge\n{}",
        run.stderr
    );
}

#[test]
fn an_export_the_change_moved_from_another_file_is_not_new() {
    // The function leaves one file and arrives in another in the same change.
    // Whatever called it before still calls it, wherever the search looked.
    let repo = Repo::init();
    let moved = fixture_file("S4", "ts", "fire/after", "src/text.ts");
    repo.write("src/words.ts", &moved);
    repo.write(
        "src/text.test.ts",
        &fixture_file("S4", "ts", "fire/after", "src/text.test.ts").replace("./text", "./words"),
    );
    repo.commit("the export lives in words");
    repo.remove("src/words.ts");
    repo.write("src/text.ts", &moved);
    repo.write(
        "src/text.test.ts",
        &fixture_file("S4", "ts", "fire/after", "src/text.test.ts"),
    );
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    assert_eq!(
        run.findings(),
        Vec::new(),
        "a move is not new code\n{}",
        run.stderr
    );
}

#[test]
fn a_change_that_makes_a_definition_exported_is_judged_like_one_that_adds_it() {
    let repo = Repo::init();
    let exported = fixture_file("S4", "ts", "fire/after", "src/text.ts");
    repo.write(
        "src/text.ts",
        &exported.replace("export function slugify", "function slugify"),
    );
    repo.commit("a private helper");
    repo.write("src/text.ts", &exported);
    repo.write(
        "src/text.test.ts",
        &fixture_file("S4", "ts", "fire/after", "src/text.test.ts"),
    );
    repo.stage_all();

    let run = repo.weeder(&["check"]);
    let findings = run.findings();
    assert_eq!(
        findings
            .iter()
            .map(|finding| finding.rule.as_str())
            .collect::<Vec<_>>(),
        vec!["S4"],
        "exporting a helper for its tests alone is the same shape\n{}",
        run.stderr
    );
}

#[test]
fn s4_is_allowed_by_a_trailer_that_carries_a_reason() {
    let repo = fixture("S4", "ts", "fire");
    repo.pending_message(
        "Add slugify ahead of the links slice\n\nWeeder-allow: S4 the links slice wires it next\n",
    );

    let run = repo.weeder(&["check"]);
    let findings = run.findings();
    assert!(
        !findings.is_empty() && findings.iter().all(|finding| finding.suppressed),
        "the trailer stands the finding down: {findings:?}"
    );
    assert_eq!(run.code, 0);
}

/// A repository whose change is the TypeScript fire fixture's, built here so a
/// test can alter it before judging.
fn typescript_repo() -> Repo {
    let repo = Repo::init();
    write_all(&repo, &fixture_tree("fire/before"));
    repo.commit("the state the change starts from");
    write_all(&repo, &fixture_tree("fire/after"));
    repo.stage_all();
    repo
}

/// The files of one side of the TypeScript fire fixture, as the repository holds them.
fn fixture_tree(side: &str) -> Vec<(&'static str, String)> {
    let paths: &[&'static str] = if side.ends_with("before") {
        &["src/text.ts", "src/app.ts"]
    } else {
        &["src/text.ts", "src/app.ts", "src/text.test.ts"]
    };
    paths
        .iter()
        .map(|path| (*path, fixture_file("S4", "ts", side, path)))
        .collect()
}

fn write_all(repo: &Repo, files: &[(&str, String)]) {
    for (path, text) in files {
        repo.write(path, text);
    }
}

/// The line the fire fixture declares `name` on, counted from one.
fn declared_at(lang: &str, path: &str, name: &str) -> u64 {
    let after = fixture_file("S4", lang, "fire/after", path);
    after
        .lines()
        .position(|line| {
            let code = line.trim_start();
            (code.starts_with("export function ")
                || code.starts_with("def ")
                || code.starts_with("pub fn ")
                || code.starts_with("func "))
                && code.contains(&format!("{name}("))
        })
        .map(|index| index as u64 + 1)
        .unwrap_or_else(|| panic!("{lang}: the fixture declares `{name}` in {path}"))
}
