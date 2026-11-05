//! `[scope] exclude` and `[[overrides]]`, the paths a repository takes away
//! from the rules and the rule levels it sets for some paths only (#99, #85).
//!
//! Any repository keeps files that look dishonest on purpose: fixtures, the
//! recorded runs of other projects, a frozen old tree. It says so the way every
//! code-quality tool lets it, with gitignore-style patterns read from its root.
//! What keeps that from being a way round the gate is written down in #99 and
//! held here: the law comes from the base, `weeder.toml` is a guardrail C1
//! blocks, a file moved into an excluded path is the deletion of the path it
//! left, and the patterns that would let a gate be told not to read its own law
//! are refused with exit 3 and the way forward.
//!
//! Every pattern that set a file aside is reported once, at note level, with a
//! count, by `check` and by `scan`, in the SARIF log as in the table.

mod common;

use common::{ours, separator, theirs, Finding, Repo, BRANCH};

/// The id the note carries.
const NOTICE: &str = "EXCLUDED";

/// One file per language weeder reads, each holding a conflict git could not
/// merge, which G1 blocks wherever it sits.
const LANGUAGES: &[(&str, &str)] = &[
    ("ts", "export const parse = (input: string) => input;"),
    ("py", "def parse(text):\n    return text"),
    ("rs", "pub fn parse(input: &str) -> &str {\n    input\n}"),
    (
        "go",
        "package parse\n\nfunc Parse(input string) string {\n\treturn input\n}",
    ),
];

fn conflicted(body: &str) -> String {
    format!(
        "{opener}\n{body}\n{separator}\n{body}\n{closer}\n",
        opener = ours("HEAD"),
        separator = separator(),
        closer = theirs(BRANCH),
    )
}

/// A `weeder.toml` stating one `[scope] exclude` list and nothing else.
fn excluding(patterns: &[&str]) -> String {
    let listed: Vec<String> = patterns.iter().map(|one| format!("{one:?}")).collect();
    format!("[scope]\nexclude = [{}]\n", listed.join(", "))
}

/// A repository whose base states `config`, with the same conflicted file in
/// each language written twice, under `test/fixtures/` and under `src/`.
fn repository(config: &str) -> Repo {
    let repo = Repo::init();
    repo.write("weeder.toml", config);
    repo.write("README.md", "# demo\n");
    repo.commit("the state the change starts from");
    for (extension, body) in LANGUAGES {
        repo.write(
            &format!("test/fixtures/parse.{extension}"),
            &conflicted(body),
        );
        repo.write(&format!("src/parse.{extension}"), &conflicted(body));
    }
    repo.stage_all();
    repo
}

fn under<'a>(found: &'a [Finding], prefix: &str) -> Vec<&'a Finding> {
    found
        .iter()
        .filter(|finding| finding.path.starts_with(prefix))
        .collect()
}

fn notes(found: &[Finding]) -> Vec<&Finding> {
    found
        .iter()
        .filter(|finding| finding.rule == NOTICE)
        .collect()
}

#[test]
fn an_excluded_directory_is_judged_by_no_rule_in_any_language_and_says_so_once() {
    let repo = repository(&excluding(&["test/fixtures/"]));
    let run = repo.weeder(&["check", "--format", "sarif"]);
    let found = run.findings();

    let inside: Vec<&Finding> = under(&found, "test/fixtures/")
        .into_iter()
        .filter(|finding| finding.rule != NOTICE)
        .collect();
    assert!(
        inside.is_empty(),
        "no rule reads a file the pattern covers: {inside:#?}"
    );
    let noted = notes(&found);
    assert_eq!(noted.len(), 1, "one note for the pattern: {found:#?}");
    assert_eq!(noted[0].level, "note");
    assert_eq!(
        noted[0].path, "test/fixtures/",
        "the note names the pattern"
    );
    assert!(
        noted[0].message.contains("4 files"),
        "and counts the files it set aside: {}",
        noted[0].message
    );

    for (extension, _) in LANGUAGES {
        let path = format!("src/parse.{extension}");
        assert!(
            found
                .iter()
                .any(|finding| finding.path == path && finding.rule == "G1"),
            "{path} is outside the pattern and judged as before: {found:#?}"
        );
    }
    assert_eq!(
        run.code, 2,
        "and the change still blocks on what it did not cover"
    );

    let table = repo.weeder(&["check", "--format", "table"]);
    assert_eq!(
        table.stdout.matches(NOTICE).count(),
        1,
        "the table carries the one note:\n{}",
        table.stdout
    );
}

#[test]
fn every_pattern_form_a_reader_expects_covers_what_gitignore_would() {
    for (pattern, covered) in [
        ("test/fixtures", 4),
        ("test/fixtures/**", 4),
        ("test/*/parse.ts", 1),
        ("**/*.py", 2),
        ("test/**/*.rs", 1),
    ] {
        let repo = repository(&excluding(&[pattern]));
        let run = repo.weeder(&["check", "--format", "sarif"]);
        let found = run.findings();
        let noted = notes(&found);
        assert_eq!(noted.len(), 1, "{pattern}: {found:#?}");
        let files = if covered == 1 { "file" } else { "files" };
        assert!(
            noted[0].message.contains(&format!("{covered} {files}")),
            "{pattern} covers {covered}: {}",
            noted[0].message
        );
    }
}

#[test]
fn each_pattern_is_noted_with_its_own_count_and_a_pattern_that_took_nothing_is_silent() {
    let repo = repository(&excluding(&["test/fixtures/", "src/*.go", "nothing/here/"]));
    let run = repo.weeder(&["check", "--format", "sarif"]);
    let found = run.findings();
    let mut noted: Vec<(String, String)> = notes(&found)
        .into_iter()
        .map(|finding| (finding.path.clone(), finding.message.clone()))
        .collect();
    noted.sort();
    assert_eq!(
        noted.len(),
        2,
        "one note per pattern that set a file aside: {noted:#?}"
    );
    assert_eq!(noted[0].0, "src/*.go");
    assert!(noted[0].1.contains("1 file"), "{}", noted[0].1);
    assert_eq!(noted[1].0, "test/fixtures/");
    assert!(noted[1].1.contains("4 files"), "{}", noted[1].1);
}

#[test]
fn scan_sets_the_same_paths_aside_and_says_so_once() {
    let repo = Repo::init();
    let marked = "// TODO: split the record on the separator the header names\nexport const parse = (line: string) => line.split(\",\");\n";
    repo.write("weeder.toml", &excluding(&["test/fixtures/"]));
    repo.write("src/parse.ts", marked);
    repo.write("test/fixtures/one/parse.ts", marked);
    repo.write("test/fixtures/two/parse.ts", marked);
    repo.commit_dated("the markers are written", "@1600000000 +0000");

    let run = repo.weeder(&["scan", "--format", "sarif"]);
    assert_eq!(run.code, 0, "a scan never blocks: {}", run.stderr);
    let found = run.findings();
    let inside: Vec<&Finding> = under(&found, "test/fixtures/")
        .into_iter()
        .filter(|finding| finding.rule != NOTICE)
        .collect();
    assert!(inside.is_empty(), "no scan rule read them: {inside:#?}");
    let noted = notes(&found);
    assert_eq!(noted.len(), 1, "{found:#?}");
    assert!(noted[0].message.contains("2 files"), "{}", noted[0].message);
    assert!(
        found
            .iter()
            .any(|finding| finding.path == "src/parse.ts" && finding.rule == "R3"),
        "the marker outside the pattern is still reported: {found:#?}"
    );
}

#[test]
fn an_exclusion_a_change_adds_does_not_apply_to_that_change_and_c1_blocks_it() {
    let repo = Repo::init();
    repo.write("README.md", "# demo\n");
    repo.commit("a base with no law");
    repo.write("weeder.toml", &excluding(&["src/"]));
    repo.write("src/parse.ts", &conflicted(LANGUAGES[0].1));
    repo.stage_all();

    let run = repo.weeder(&["check", "--format", "sarif"]);
    let found = run.findings();
    assert_eq!(run.code, 2, "{}", run.output());
    assert!(
        found
            .iter()
            .any(|finding| finding.path == "src/parse.ts" && finding.rule == "G1"),
        "the base carries no exclusion, so the change is judged whole: {found:#?}"
    );
    assert!(
        found.iter().any(|finding| finding.path == "weeder.toml"
            && finding.rule == "C1"
            && finding.level == "error"),
        "and the exclusion itself is a guardrail edit a person signs: {found:#?}"
    );
    assert!(
        notes(&found).is_empty(),
        "nothing was set aside: {found:#?}"
    );
}

#[test]
fn a_test_moved_into_an_excluded_path_is_still_a_test_deleted() {
    let repo = Repo::init();
    repo.write("weeder.toml", &excluding(&["test/fixtures/"]));
    repo.write(
        "src/a.ts",
        "export const add = (a: number, b: number) => a + b;\n",
    );
    repo.write(
        "tests/a.test.ts",
        "import { add } from \"../src/a\";\n\n\
         describe(\"add\", () => {\n  \
         it(\"adds two numbers\", () => {\n    expect(add(1, 2)).toBe(3);\n  });\n\n  \
         it(\"adds a negative\", () => {\n    expect(add(1, -2)).toBe(-1);\n  });\n});\n",
    );
    repo.commit("a suite");
    std::fs::create_dir_all(repo.root().join("test/fixtures")).expect("the directory it moves to");
    repo.git(&["mv", "tests/a.test.ts", "test/fixtures/a.test.ts"]);

    let run = repo.weeder(&["check", "--format", "sarif"]);
    let found = run.findings();
    let t1: Vec<&Finding> = found
        .iter()
        .filter(|finding| finding.rule == "T1")
        .collect();
    assert_eq!(t1.len(), 1, "the suite lost a file: {found:#?}");
    assert_eq!(t1[0].path, "tests/a.test.ts", "named by the path it left");
    assert_eq!(t1[0].level, "error");
    assert_eq!(run.code, 2, "{}", run.output());
    let noted = notes(&found);
    assert_eq!(noted.len(), 1, "where it went is noted: {found:#?}");
    assert!(noted[0].message.contains("1 file"), "{}", noted[0].message);
}

#[test]
fn each_refusal_exits_3_naming_the_pattern_and_the_way_forward() {
    for (pattern, forward) in [
        ("/etc", "relative to the root"),
        ("C:/fixtures", "relative to the root"),
        ("~/fixtures", "relative to the root"),
        ("../elsewhere", "without `..`"),
        ("test/../src", "without `..`"),
        ("*", "name the directories or files"),
        ("**", "name the directories or files"),
        ("**/*", "name the directories or files"),
        ("/", "name the directories or files"),
        ("", "write the directory or files"),
        ("weeder.toml", "narrow the pattern"),
        ("AGENTS.md", "narrow the pattern"),
        ("**/*.md", "narrow the pattern"),
        (".githooks/", "narrow the pattern"),
        (".github/", "narrow the pattern"),
        (".github/workflows/ci.yml", "narrow the pattern"),
        (".claude/**", "narrow the pattern"),
    ] {
        for face in ["check", "scan"] {
            let repo = repository(&excluding(&[pattern]));
            let run = repo.weeder(&[face, "--format", "sarif"]);
            assert_eq!(
                run.code, 3,
                "{face}: a pattern weeder cannot allow is a run that never happened: `{pattern}`"
            );
            assert!(
                run.stderr.contains(&format!("`{pattern}`")),
                "{face}: the refusal names the pattern `{pattern}`: {}",
                run.stderr
            );
            assert!(
                run.stderr.contains(forward),
                "{face}: and the way forward for `{pattern}`, `{forward}`: {}",
                run.stderr
            );
        }
    }
}

#[test]
fn a_pattern_that_reaches_a_guardrail_by_a_wildcard_leaves_the_guardrail_judged() {
    let repo = repository(&excluding(&["**/fixtures/"]));
    repo.write(".githooks/fixtures/post-commit", "#!/bin/sh\nexit 0\n");
    repo.stage_all();
    let run = repo.weeder(&["check", "--format", "sarif"]);
    let found = run.findings();
    assert!(
        found.iter().any(|finding| finding.rule == "C1"
            && finding.path == ".githooks/fixtures/post-commit"),
        "a hook git can run is judged whatever pattern reaches it: {found:#?}"
    );
    let noted = notes(&found);
    assert_eq!(noted.len(), 1, "{found:#?}");
    assert!(
        noted[0].message.contains("4 files"),
        "the pattern set aside the fixtures and not the hook: {}",
        noted[0].message
    );
}

#[test]
fn a_pattern_over_a_path_the_repository_holds_as_a_guardrail_is_refused() {
    let repo = repository(&format!(
        "{}\n[guardrails]\npaths = [\"policy/**\"]\n",
        excluding(&["policy/"])
    ));
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(run.code, 3, "{}", run.output());
    assert!(run.stderr.contains("`policy/`"), "{}", run.stderr);
}

#[test]
fn the_old_key_is_gone_and_the_refusal_names_the_new_one() {
    let repo = repository("[scope]\nspecimens = [\"test/fixtures\"]\n");
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(run.code, 3, "no alias is kept: {}", run.output());
    assert!(
        run.stderr.contains("specimens") && run.stderr.contains("exclude"),
        "the refusal names what was written and what is read now: {}",
        run.stderr
    );
}

/// A repository with a conflicted file in three places: `src/`, `docs/` and
/// `docs/keep/`, under whatever overrides the base states.
fn overridden(config: &str) -> Repo {
    let repo = Repo::init();
    repo.write("weeder.toml", config);
    repo.write("README.md", "# demo\n");
    repo.commit("the law");
    for path in ["src/parse.ts", "docs/notes.md", "docs/keep/notes.md"] {
        repo.write(path, &conflicted("a line both sides changed"));
    }
    repo.stage_all();
    repo
}

/// The level G1 reported at on each path, `-` where it reported nothing.
fn g1_levels(repo: &Repo) -> Vec<(String, String)> {
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_ne!(run.code, 3, "the run happened: {}", run.stderr);
    ["src/parse.ts", "docs/notes.md", "docs/keep/notes.md"]
        .iter()
        .map(|path| {
            let mut levels: Vec<String> = run
                .findings()
                .into_iter()
                .filter(|finding| finding.rule == "G1" && finding.path == *path)
                .map(|finding| finding.level)
                .collect();
            levels.dedup();
            let level = if levels.is_empty() {
                "-".to_string()
            } else {
                levels.join(",")
            };
            (path.to_string(), level)
        })
        .collect()
}

fn pairs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(path, level)| (path.to_string(), level.to_string()))
        .collect()
}

#[test]
fn an_override_sets_a_rule_under_its_paths_and_nowhere_else() {
    let repo = overridden("[[overrides]]\npaths = [\"docs/**\"]\nrules = { G1 = \"off\" }\n");
    assert_eq!(
        g1_levels(&repo),
        pairs(&[
            ("src/parse.ts", "error"),
            ("docs/notes.md", "-"),
            ("docs/keep/notes.md", "-"),
        ]),
        "off under docs/, on everywhere else"
    );

    let repo = overridden("[[overrides]]\npaths = [\"docs/\"]\nrules = { G1 = \"warn\" }\n");
    assert_eq!(
        g1_levels(&repo),
        pairs(&[
            ("src/parse.ts", "error"),
            ("docs/notes.md", "warning"),
            ("docs/keep/notes.md", "warning"),
        ]),
        "a level written there caps the rule there, as [rules] caps it everywhere"
    );
}

#[test]
fn the_most_specific_override_wins() {
    let repo = overridden(
        "[[overrides]]\npaths = [\"docs/**\"]\nrules = { G1 = \"off\" }\n\n\
         [[overrides]]\npaths = [\"docs/keep/\"]\nrules = { G1 = \"block\" }\n",
    );
    assert_eq!(
        g1_levels(&repo),
        pairs(&[
            ("src/parse.ts", "error"),
            ("docs/notes.md", "-"),
            ("docs/keep/notes.md", "error"),
        ]),
        "docs/keep/ is more specific than docs/**, whichever is written first"
    );

    let repo = overridden(
        "[rules]\nG1 = \"off\"\n\n\
         [[overrides]]\npaths = [\"src/\"]\nrules = { G1 = \"block\" }\n",
    );
    assert_eq!(
        g1_levels(&repo),
        pairs(&[
            ("src/parse.ts", "error"),
            ("docs/notes.md", "-"),
            ("docs/keep/notes.md", "-"),
        ]),
        "an override turns a rule on for its paths where [rules] turns it off"
    );
}

#[test]
fn two_equally_specific_overrides_that_disagree_are_refused() {
    let repo = overridden(
        "[[overrides]]\npaths = [\"docs/*\"]\nrules = { G1 = \"off\" }\n\n\
         [[overrides]]\npaths = [\"*/keep\"]\nrules = { G1 = \"warn\" }\n",
    );
    let run = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(run.code, 3, "weeder will not pick one: {}", run.output());
    for named in ["`docs/*`", "`*/keep`", "G1", "narrower"] {
        assert!(
            run.stderr.contains(named),
            "the refusal says {named}: {}",
            run.stderr
        );
    }

    let repo = overridden(
        "[[overrides]]\npaths = [\"docs/**\"]\nrules = { G1 = \"off\" }\n\n\
         [[overrides]]\npaths = [\"test/**\"]\nrules = { G1 = \"warn\" }\n",
    );
    assert_ne!(
        repo.weeder(&["check", "--format", "sarif"]).code,
        3,
        "two that can never cover one file do not conflict"
    );
}

#[test]
fn an_override_over_a_guardrail_or_naming_no_rule_is_refused() {
    for config in [
        "[[overrides]]\npaths = [\".github/**\"]\nrules = { C3 = \"off\" }\n",
        "[[overrides]]\npaths = [\"docs/**\"]\nrules = { R9 = \"off\" }\n",
        "[[overrides]]\npaths = [\"docs/**\"]\nrules = {}\n",
        "[[overrides]]\npaths = [\"docs/**\"]\nrules = { R1 = \"warn\" }\n",
    ] {
        let repo = overridden(config);
        let run = repo.weeder(&["check", "--format", "sarif"]);
        assert_eq!(run.code, 3, "refused: {config}\n{}", run.output());
    }
}

#[test]
fn an_override_turning_r1_off_under_research_silences_it_there_and_nowhere_else() {
    let cited = "# notes\n\nThe parser lives in `src/parser.ts`.\n";
    let scan = |config: Option<&str>| {
        let repo = Repo::init();
        if let Some(config) = config {
            repo.write("weeder.toml", config);
        }
        repo.write("docs/research/elsewhere.md", cited);
        repo.write("docs/guide.md", cited);
        repo.commit("two docs citing a file that is not here");
        let run = repo.weeder(&["scan", "--rules", "R1", "--format", "sarif"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        let mut paths: Vec<String> = run
            .findings()
            .into_iter()
            .filter(|finding| finding.rule == "R1")
            .map(|finding| finding.path)
            .collect();
        paths.dedup();
        paths
    };
    assert_eq!(
        scan(None),
        vec![
            "docs/guide.md".to_string(),
            "docs/research/elsewhere.md".to_string()
        ],
        "both docs cite a file that is gone"
    );
    assert_eq!(
        scan(Some(
            "[[overrides]]\npaths = [\"docs/research/**\"]\nrules = { R1 = \"off\" }\n"
        )),
        vec!["docs/guide.md".to_string()],
        "the research doc describes another codebase, and R1 leaves it alone"
    );
}
