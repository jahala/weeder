//! D2, an import crossed a boundary the repository forbids.
//!
//! `[deps]` names the layers a repository is built from and the directions an
//! import may run between them. Everything else is an arrow somebody drew the
//! wrong way, and it is the kind of change that reads as one line and takes a
//! week to undo.
//!
//! weed holds itself to this rule: core computes and reaches nothing, the seams
//! reach the world, the faces put the two together. The last case here copies
//! weed's own source into a repository, finds nothing, and then draws one arrow
//! backwards.

mod common;

use std::path::Path;

use common::{fixture, Repo};

/// The file that reaches across the boundary, one language at a time, and the
/// file the neighbour reaches from, which is allowed to.
const LAYERS: [(&str, &str, &str); 4] = [
    ("ts", "src/core/finding.ts", "src/faces/check.ts"),
    ("py", "src/core/finding.py", "src/faces/check.py"),
    ("rs", "src/core/finding.rs", "src/faces/check.rs"),
    ("go", "core/finding.go", "faces/check.go"),
];

/// The layers the fixtures and weed's own `weed.toml` name.
const CORE: &str = "core";
const SEAMS: &str = "seams";

#[test]
fn d2_fires_at_block_level_where_a_layer_reaches_one_it_may_not_in_every_language() {
    for (lang, offender, _) in LAYERS {
        let repo = fixture("D2", lang, "fire");
        let run = repo.weed(&["check"]);

        assert_eq!(
            run.code, 2,
            "{lang}: a forbidden direction blocks\n{}",
            run.stderr
        );
        let findings = run.findings();
        assert_eq!(findings.len(), 1, "{lang}: one import, one finding");
        let finding = &findings[0];
        assert_eq!(finding.rule, "D2", "{lang}: the rule is D2");
        assert_eq!(finding.level, "error", "{lang}: D2 blocks");
        assert_eq!(
            finding.path, offender,
            "{lang}: the finding names the file that reached"
        );
        assert!(
            finding.message.contains(CORE) && finding.message.contains(SEAMS),
            "{lang}: the message names both ends of the boundary: {}",
            finding.message
        );
        assert!(
            finding.line.is_some_and(|line| line >= 1),
            "{lang}: the finding lands on the import"
        );
    }
}

#[test]
fn d2_stays_silent_on_a_direction_the_repository_allows() {
    for (lang, _, allowed) in LAYERS {
        let repo = fixture("D2", lang, "silent");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|line| line == allowed),
            "{lang}: {allowed} must be in the diff, or the silence proves nothing"
        );

        let run = repo.weed(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{lang}: a face reaching a seam is the direction the arrows run in"
        );
        assert_eq!(run.code, 0, "{lang}: nothing found, nothing blocked");
    }
}

#[test]
fn weeds_own_source_holds_its_doctrine_until_one_arrow_is_drawn_backwards() {
    let repo = weeds_own_source();
    let clean = repo.weed(&["check"]);
    assert_eq!(
        d2(&clean),
        Vec::<String>::new(),
        "every import in weed's own source runs the way weed's own weed.toml allows\n{}",
        clean.stderr
    );

    // One line, in the layer that is supposed to reach nothing.
    let core = "src/core/finding.rs";
    let source = std::fs::read_to_string(repo.root().join(core)).expect("the core file reads");
    repo.write(
        core,
        &format!("use crate::{SEAMS}::git::GitError;\n{source}"),
    );
    repo.stage_all();

    let broken = repo.weed(&["check"]);
    assert_eq!(
        d2(&broken),
        vec![core.to_string()],
        "the one file that reaches across is named, and no other\n{}",
        broken.stderr
    );
    assert_eq!(broken.code, 2, "a forbidden direction blocks");
}

/// A repository holding weed's own source and weed's own `weed.toml`, with all
/// of it staged as the change to judge.
fn weeds_own_source() -> Repo {
    let repo = Repo::init();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    copy_tree(&root.join("src"), &repo.root().join("src"));
    let config = std::fs::read_to_string(root.join("weed.toml")).expect("weed states its own law");
    repo.write("weed.toml", &config);
    repo.stage_all();
    repo
}

/// The paths D2 named, sorted and without repeats.
fn d2(run: &common::Run) -> Vec<String> {
    let mut paths: Vec<String> = run
        .findings()
        .into_iter()
        .filter(|finding| finding.rule == "D2")
        .map(|finding| finding.path)
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).expect("a directory for the copy");
    for entry in std::fs::read_dir(source).expect("weed's own source should be readable") {
        let entry = entry.expect("a source entry should be readable");
        let from = entry.path();
        let to = target.join(entry.file_name());
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            std::fs::copy(&from, &to).expect("a source file should be copyable");
        }
    }
}

/// A relative import that climbs more than one directory is the ordinary way a
/// suite or a nested module reaches its neighbour, and it has to resolve to the
/// file it names. Read as anything else, the arrow lands in the wrong layer:
/// the import is missed where it crosses, and reported where it does not.
#[test]
fn d2_follows_a_relative_import_up_more_than_one_directory() {
    let repo = Repo::init();
    repo.write(
        "weed.toml",
        "[deps]\nlayers = { core = [\"src/core/**\"], seams = [\"src/seams/**\"] }\nallow = [\n    { from = \"seams\", to = \"core\" },\n]\n",
    );
    repo.write(
        "src/seams/git.ts",
        "export function read(path: string) {\n  return path;\n}\n",
    );
    repo.write(
        "src/core/render/finding.ts",
        "export function render(rule: string): string {\n  return rule;\n}\n",
    );
    repo.stage_all();
    repo.commit("the layers, with every arrow running the way they allow");

    let climbing = "src/core/render/finding.ts";
    let source = std::fs::read_to_string(repo.root().join(climbing)).expect("the file reads");
    repo.write(
        climbing,
        &format!("import {{ read }} from \"../../seams/git\";\n{source}"),
    );
    repo.stage_all();

    let run = repo.weed(&["check"]);
    assert_eq!(
        d2(&run),
        vec![climbing.to_string()],
        "a `../../` import reaches the file it names, and that one crosses the boundary\n{}",
        run.stderr
    );
    assert_eq!(run.code, 2, "a forbidden direction blocks");
}

/// A climb of three, with a sibling directory sitting where a wrongly-read
/// climb would land. Counting the dots rather than walking the segments puts
/// the module two directories from where it is, and the path that answers
/// there is in the other layer, so the two answers cancel and the arrow
/// disappears. The import has to resolve where it points.
#[test]
fn d2_follows_a_climb_past_a_sibling_that_would_answer_a_misread_one() {
    let repo = Repo::init();
    repo.write(
        "weed.toml",
        "[deps]\nlayers = { suite = [\"test/**\"], tools = [\"scripts/**\"] }\nallow = []\n",
    );
    repo.write(
        "scripts/build.ts",
        "export function build() {\n  return 1;\n}\n",
    );
    repo.write("test/smoke/helpers.ts", "export const HERE = 1;\n");
    repo.write(
        "test/smoke/claude/resume.test.ts",
        "import { HERE } from \"../helpers\";\n\nexport const seen = HERE;\n",
    );
    repo.stage_all();
    repo.commit("a suite, its helpers, and the tools beside them");

    let climbing = "test/smoke/claude/resume.test.ts";
    let source = std::fs::read_to_string(repo.root().join(climbing)).expect("the file reads");
    repo.write(
        climbing,
        &format!("import {{ build }} from \"../../../scripts/build\";\n{source}"),
    );
    repo.stage_all();

    let run = repo.weed(&["check"]);
    assert_eq!(
        d2(&run),
        vec![climbing.to_string()],
        "the import names scripts/build.ts, three directories up, and that crosses\n{}",
        run.stderr
    );
    assert_eq!(run.code, 2, "a forbidden direction blocks");
}

/// The same climb, in the direction the repository allows: an import that
/// resolves correctly is silent, and one resolved into the wrong layer would
/// not be.
#[test]
fn d2_stays_silent_where_the_climb_runs_the_way_the_layers_allow() {
    let repo = Repo::init();
    repo.write(
        "weed.toml",
        "[deps]\nlayers = { core = [\"src/core/**\"], seams = [\"src/seams/**\"] }\nallow = [\n    { from = \"seams\", to = \"core\" },\n]\n",
    );
    repo.write(
        "src/core/finding.ts",
        "export function render(rule: string): string {\n  return rule;\n}\n",
    );
    repo.write(
        "src/seams/git/reader.ts",
        "export function read(path: string) {\n  return path;\n}\n",
    );
    repo.stage_all();
    repo.commit("the layers");

    let climbing = "src/seams/git/reader.ts";
    let source = std::fs::read_to_string(repo.root().join(climbing)).expect("the file reads");
    repo.write(
        climbing,
        &format!("import {{ render }} from \"../../core/finding\";\n{source}"),
    );
    repo.stage_all();

    let run = repo.weed(&["check"]);
    assert_eq!(
        d2(&run),
        Vec::<String>::new(),
        "a seam reaching the core is the direction the arrows run in\n{}",
        run.stderr
    );
}
