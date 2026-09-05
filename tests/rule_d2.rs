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
