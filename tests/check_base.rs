//! `weeder check --base <ref>` judges a branch the way a pull request shows it:
//! against the merge-base of the ref and HEAD, which is what `git diff <ref>...`
//! reads (#167). A branch that has fallen behind its base carries none of what
//! the base did since the branch was cut, and judging it against the base's own
//! tree would read every one of those changes as the branch undoing them.

mod common;

use common::Repo;

const FORMAT: &str = "export function format(value: string, width: number): string {
  return value.padEnd(width);
}
";

const FORMAT_TEST: &str = "import { format } from \"./format\";

describe(\"format\", () => {
  it(\"pads to the width\", () => {
    expect(format(\"a\", 3)).toBe(\"a  \");
    expect(format(\"ab\", 3)).toBe(\"ab \");
  });
});
";

const FORMAT_TEST_GROWN: &str = "import { format } from \"./format\";

describe(\"format\", () => {
  it(\"pads to the width\", () => {
    expect(format(\"a\", 3)).toBe(\"a  \");
    expect(format(\"ab\", 3)).toBe(\"ab \");
  });

  it(\"leaves a full value alone\", () => {
    expect(format(\"abc\", 3)).toBe(\"abc\");
    expect(format(\"abcd\", 3)).toBe(\"abcd\");
  });
});
";

const PARSE: &str = "export const parse = (input: string) => input.trim();
";

const PARSE_TEST: &str = "import { parse } from \"./parse\";

describe(\"parse\", () => {
  it(\"trims the left\", () => {
    expect(parse(\"  a\")).toBe(\"a\");
    expect(parse(\"   ab\")).toBe(\"ab\");
  });

  it(\"trims the right\", () => {
    expect(parse(\"a  \")).toBe(\"a\");
    expect(parse(\"ab   \")).toBe(\"ab\");
  });
});
";

const PARSE_TEST_SHRUNK: &str = "import { parse } from \"./parse\";

describe(\"parse\", () => {
  it(\"trims the left\", () => {
    expect(parse(\"  a\")).toBe(\"a\");
    expect(parse(\"   ab\")).toBe(\"ab\");
  });
});
";

/// A `main` and a `feature` cut from it, then `main` moved on by a test the
/// feature never saw. The checkout is left on `feature`, with the feature's own
/// commit holding whatever `feature_writes` put in the tree.
fn a_branch_behind_its_base(feature_writes: impl Fn(&Repo)) -> Repo {
    let repo = Repo::init();
    repo.write("src/format.ts", FORMAT);
    repo.write("src/format.test.ts", FORMAT_TEST);
    repo.write("src/parse.ts", PARSE);
    repo.write("src/parse.test.ts", PARSE_TEST);
    repo.commit("the suite begins");

    repo.git(&["switch", "--quiet", "-c", "feature"]);
    feature_writes(&repo);
    repo.commit("the feature's own change");

    repo.git(&["switch", "--quiet", "main"]);
    repo.write("src/format.test.ts", FORMAT_TEST_GROWN);
    repo.commit("main grows a test the feature never saw");

    repo.git(&["switch", "--quiet", "feature"]);
    repo
}

fn on(run: &common::Run, path: &str) -> Vec<String> {
    run.findings()
        .into_iter()
        .filter(|finding| finding.path == path)
        .map(|finding| finding.rule)
        .collect()
}

#[test]
fn a_branch_behind_its_base_is_not_judged_for_what_the_base_did_since() {
    let repo = a_branch_behind_its_base(|repo| {
        repo.write(
            "src/format.ts",
            "export function format(value: string, width: number): string {\n  return value.padEnd(width, \" \");\n}\n",
        );
    });

    let run = repo.weeder(&["check", "--base", "main", "--strict", "--format", "sarif"]);

    assert_eq!(
        on(&run, "src/format.test.ts"),
        Vec::<String>::new(),
        "the feature never touched the test main grew, so nothing of it is the feature's to answer for: {}",
        run.output()
    );
    assert_eq!(run.code, 0, "{}", run.output());
}

#[test]
fn a_test_the_branch_did_shrink_is_still_judged() {
    let repo = a_branch_behind_its_base(|repo| {
        repo.write("src/parse.test.ts", PARSE_TEST_SHRUNK);
    });

    let run = repo.weeder(&["check", "--base", "main", "--strict", "--format", "sarif"]);

    let parse = on(&run, "src/parse.test.ts");
    assert!(
        parse.iter().any(|rule| rule == "T1" || rule == "T2"),
        "the test the feature took out is the feature's own change: {}",
        run.output()
    );
    assert_eq!(
        on(&run, "src/format.test.ts"),
        Vec::<String>::new(),
        "and main's newer test is still nobody's finding here: {}",
        run.output()
    );
    assert_eq!(run.code, 2, "{}", run.output());
}

#[test]
fn a_base_that_shares_no_history_with_head_is_a_run_that_never_happened() {
    let repo = a_branch_behind_its_base(|repo| {
        repo.write("src/parse.test.ts", PARSE_TEST_SHRUNK);
    });
    repo.git(&["switch", "--quiet", "--orphan", "stray"]);
    repo.write("README.md", "a history of its own\n");
    repo.commit("a history that shares nothing with main");
    repo.git(&["switch", "--quiet", "feature"]);

    let run = repo.weeder(&["check", "--base", "stray", "--format", "sarif"]);

    assert_eq!(run.code, 3, "{}", run.output());
    assert_eq!(run.stderr_lines().len(), 1, "one line, naming the cause");
    assert!(
        run.stderr.contains("stray") && run.stderr.contains("HEAD"),
        "the line names both refs: {}",
        run.stderr
    );
    assert!(
        run.stderr.contains("share no commit"),
        "the line says why there is nothing to judge from: {}",
        run.stderr
    );
    assert!(
        !run.log()["runs"][0]["invocations"][0]["executionSuccessful"]
            .as_bool()
            .expect("the invocation says whether weeder ran"),
        "the log says the run never judged anything"
    );
}

/// The push form of #167, which the pre-push hook never had: its base is
/// always a commit in the pushed tip's own history. A branch cut before main
/// grew a test is pushed while it is behind, and judged from where its
/// unpublished commits begin; it merges main in and is pushed again, and is
/// judged from the tip the remote already holds, so main's test arrives the way
/// main wrote it, as a case added. Neither push reads it backwards.
#[test]
fn a_push_of_a_branch_behind_main_is_judged_on_what_it_carries() {
    let remote = Repo::bare();
    let repo = Repo::init();
    repo.weeder(&["guard", "install", "--hooks-dir", ".githooks"]);
    repo.commit("weeder guard installed");
    repo.write("src/parse.ts", PARSE);
    repo.write("src/parse.test.ts", PARSE_TEST_SHRUNK);
    repo.commit("the parser and its suite");
    let origin = remote.root().display().to_string();
    repo.git(&["remote", "add", "origin", &origin]);
    repo.git(&["push", "--quiet", "--set-upstream", "origin", "main"]);

    repo.git(&["switch", "--quiet", "-c", "feature/split"]);
    repo.write(
        "src/parse.ts",
        "export const parse = (input: string) => input.trim().toLowerCase();\n",
    );
    repo.commit("the split");
    repo.git(&["switch", "--quiet", "main"]);
    repo.write("src/parse.test.ts", PARSE_TEST);
    repo.commit("the suite grows a case the split never saw");
    let grown = repo.try_git(&["push", "origin", "main"]);
    assert_eq!(grown.code, 0, "main moves on: {}", grown.output());
    repo.git(&["switch", "--quiet", "feature/split"]);

    let behind = repo.try_git(&["push", "origin", "feature/split"]);
    assert_eq!(
        behind.code,
        0,
        "a branch behind main carries none of main's newer test: {}",
        behind.output()
    );
    assert!(
        !behind.output().contains("parse.test.ts"),
        "nothing is said about a file the branch never touched: {}",
        behind.output()
    );

    repo.git(&["merge", "--quiet", "--no-edit", "main"]);
    let merged = repo.try_git(&["push", "origin", "feature/split"]);
    assert_eq!(
        merged.code,
        0,
        "main's test arrives as the case main added: {}",
        merged.output()
    );
    assert!(
        !merged.output().contains("parse.test.ts"),
        "and draws no finding: {}",
        merged.output()
    );

    // The hook is judging all along: a case the branch takes out itself, past
    // the commit hooks, is the push's own finding.
    repo.write("src/parse.test.ts", PARSE_TEST_SHRUNK);
    repo.stage_all();
    repo.git(&[
        "commit",
        "--quiet",
        "--no-verify",
        "-m",
        "the split drops a case",
    ]);
    let shrunk = repo.try_git(&["push", "origin", "feature/split"]);
    assert_ne!(shrunk.code, 0, "{}", shrunk.output());
    assert!(
        shrunk.output().contains("parse.test.ts"),
        "the finding names the file the branch shrank: {}",
        shrunk.output()
    );
}
