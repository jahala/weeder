//! S2, an error was swallowed.
//!
//! Every language has a way of catching a failure and then doing nothing with
//! it: an empty `catch`, a bare `except` over `pass`, a nil check that returns
//! nothing, an error assigned to the name that means "throw this away", a
//! fallible call turned into a default. Each one is a line where the program
//! learns something went wrong and forgets it again.
//!
//! The neighbour is the same handler doing its job: the failure logged, wrapped
//! into one the caller can read, raised on, or dropped under a comment that says
//! why dropping it is right.

mod common;

use common::{fixture, fixture_file};

/// The shapes the check names, one language at a time: the file the change
/// touches, and the line each swallowed failure is written on. The lines are
/// read back out of the fixture, so the expectation is the fixture rather than
/// a list of numbers that drifts the moment a line is added above it.
const SHAPES: [(&str, &str, &[&str]); 5] = [
    (
        "ts",
        "src/client.ts",
        &[
            "} catch (error) {",
            "return drain().catch(() => {});",
            // The same handler written on one line, with and without its
            // binding, and with a block after it (#75).
            "try { JSON.parse(text); } catch {}",
            "try { JSON.parse(text); } catch (error) {}",
            "try { JSON.parse(text); } catch {} finally { post(text); }",
        ],
    ),
    ("py", "src/client.py", &["except Exception:", "except:"]),
    (
        "rs",
        "src/client.rs",
        &["post(payload).unwrap_or_default()", "read().ok()"],
    ),
    (
        "go",
        "client.go",
        &["if err := post(payload); err != nil {", "_ = err"],
    ),
    // The shell answers a failed command with one that cannot fail, having
    // thrown the command's own error output away first (#79). A `|| true`
    // whose stderr still shows tells the reader what went wrong.
    (
        "sh",
        "scripts/sync.sh",
        &[
            "rsync -a \"$1\" \"$2\" >/dev/null 2>&1 || true",
            "rm -r \"$2/.cache\" 2>/dev/null || :",
            "[ -d \"$2\" ] && touch \"$2/.synced\" &>/dev/null ||true",
        ],
    ),
];

#[test]
fn s2_warns_on_every_swallowed_error_shape_in_every_language() {
    for (lang, path, shapes) in SHAPES {
        let repo = fixture("S2", lang, "fire");
        let run = repo.weeder(&["check"]);

        assert_eq!(
            run.code, 0,
            "{lang}: a swallowed error is a warning, and a warning does not block\n{}",
            run.stderr
        );
        let findings = run.findings();
        let lines: Vec<u64> = findings
            .iter()
            .map(|finding| finding.line.expect("a swallowed error names its line"))
            .collect();
        assert_eq!(
            lines,
            shape_lines(lang, path, shapes),
            "{lang}: every swallowed shape is reported, and only those"
        );

        let before = fixture_file("S2", lang, "fire/before", path);
        let after = fixture_file("S2", lang, "fire/after", path);
        for finding in &findings {
            assert_eq!(finding.rule, "S2", "{lang}: the rule is S2");
            assert_eq!(finding.level, "warning", "{lang}: S2 warns");
            assert_eq!(finding.path, path, "{lang}: the finding names the file");
            let line = after
                .lines()
                .nth(finding.line.unwrap_or_default() as usize - 1)
                .expect("the reported line is a line of the file");
            assert!(
                !before.lines().any(|old| old == line),
                "{lang}: the reported line has to be one the change wrote: {line}"
            );
        }
    }
}

#[test]
fn s2_stays_silent_when_the_failure_is_logged_wrapped_raised_on_or_explained() {
    for (lang, path, _) in SHAPES {
        let repo = fixture("S2", lang, "silent");
        let changed = repo.git(&["diff", "HEAD", "--name-only"]);
        assert!(
            changed.lines().any(|changed| changed.ends_with(path)),
            "{lang}: the silent neighbour must be in the diff, or the silence proves nothing"
        );

        let run = repo.weeder(&["check"]);
        assert_eq!(
            run.findings(),
            Vec::new(),
            "{lang}: a failure that is read, passed on or explained is handled"
        );
        assert_eq!(run.code, 0, "{lang}: nothing found, nothing blocked");
    }
}

/// Where the fixture writes each shape, counted from one.
fn shape_lines(lang: &str, path: &str, shapes: &[&str]) -> Vec<u64> {
    let after = fixture_file("S2", lang, "fire/after", path);
    let mut lines: Vec<u64> = shapes
        .iter()
        .map(|shape| {
            after
                .lines()
                .position(|line| line.trim() == *shape)
                .map(|index| index as u64 + 1)
                .unwrap_or_else(|| panic!("{lang}: the fixture writes `{shape}` in {path}"))
        })
        .collect();
    lines.sort_unstable();
    lines
}

/// The next action asks for a comment saying why a failure is right to drop,
/// so a comment on the line, or alone on the line above it, is that answer, as
/// a comment inside an empty handler already is (#116). A line with no comment
/// near it is still a failure going nowhere.
#[test]
fn s2_reads_a_comment_beside_a_dropped_failure_as_the_reason_the_next_action_asks_for() {
    let cases: [(&str, &str, &str, &str); 2] = [
        (
            "src/lib.rs",
            "pub fn count(v: Option<u32>) -> u32 {\n    v.unwrap_or(0)\n}\n",
            "pub fn count(v: Option<u32>) -> u32 {\n    // no value means no subject, and zero is the count for no subject\n    v.unwrap_or_default()\n}\n\npub fn read_or_none(r: Result<u32, String>) -> Option<u32> {\n    r.ok() // a missing value is already reported upstream\n}\n",
            "pub fn count(v: Option<u32>) -> u32 {\n    v.unwrap_or_default()\n}\n",
        ),
        (
            "client.go",
            "package client\n\nfunc post() error { return nil }\n\nfunc send() {\n\tpost()\n}\n",
            "package client\n\nfunc post() error { return nil }\n\nfunc send() {\n\terr := post()\n\t// the post is a best-effort ping; its failure changes nothing here\n\t_ = err\n}\n",
            "package client\n\nfunc post() error { return nil }\n\nfunc send() {\n\terr := post()\n\t_ = err\n}\n",
        ),
    ];
    for (path, before, explained, bare) in cases {
        let repo = common::Repo::init();
        repo.write(path, before);
        repo.commit("the code before the change");

        repo.write(path, explained);
        let run = repo.weeder(&["check"]);
        assert!(
            run.findings().iter().all(|finding| finding.rule != "S2"),
            "{path}: a comment saying why is the answer S2 asks for\n{}",
            run.output()
        );

        repo.write(path, bare);
        let run = repo.weeder(&["check"]);
        assert!(
            run.findings().iter().any(|finding| finding.rule == "S2"),
            "{path}: with no comment near it the failure still goes nowhere\n{}",
            run.output()
        );
    }
}

#[test]
fn s2_reads_a_reason_written_above_the_statement_a_dropped_failure_belongs_to() {
    // A chain written over several lines drops its failure on its last line,
    // and the reason sits where people put a reason, above the statement (#130).
    // A reason above some other statement says nothing about this one. Python's
    // handler and Go's discard open a statement of their own, which the test
    // above reads already.
    let cases: [(&str, &str, &str, &str); 3] = [
        (
            "src/lib.rs",
            "pub fn first(lines: &[&str]) -> String {\n    lines[0].to_string()\n}\n",
            "pub fn first(lines: &[&str]) -> String {\n    // an empty file has no first line, and its text is empty\n    lines\n        .first()\n        .map(|line| line.trim().to_string())\n        .unwrap_or_default()\n}\n",
            "pub fn first(lines: &[&str]) -> String {\n    // the lines arrive trimmed\n    let count = lines.len();\n    lines\n        .first()\n        .map(|line| line.trim().to_string())\n        .unwrap_or_default()\n        + &count.to_string()\n}\n",
        ),
        (
            "src/client.ts",
            "export function ping(post: () => Promise<void>): void {\n  void post();\n}\n",
            "export function ping(post: () => Promise<void>): void {\n  // a best-effort ping; its failure changes nothing here\n  post()\n    .then(() => undefined)\n    .catch(() => {});\n}\n",
            "export function ping(post: () => Promise<void>): void {\n  // the ping runs once a minute\n  const started = Date.now();\n  post()\n    .then(() => started)\n    .catch(() => {});\n}\n",
        ),
        // A shell command continued with a backslash drops its failure on its
        // last line, and its reason sits above its first.
        (
            "scripts/fetch.sh",
            "#!/usr/bin/env bash\nfetch() {\n  curl -fsS \"$1\"\n}\n",
            "#!/usr/bin/env bash\nfetch() {\n  # a mirror that is down is tried again on the next run\n  curl -fsS \"$1\" \\\n    -o out 2>/dev/null || true\n}\n",
            "#!/usr/bin/env bash\nfetch() {\n  # the url arrives checked\n  local url=\"$1\"\n  curl -fsS \"$url\" \\\n    -o out 2>/dev/null || true\n}\n",
        ),
    ];
    for (path, before, explained, elsewhere) in cases {
        let repo = common::Repo::init();
        repo.write(path, before);
        repo.commit("the code before the change");

        repo.write(path, explained);
        let run = repo.weeder(&["check"]);
        assert!(
            run.findings().iter().all(|finding| finding.rule != "S2"),
            "{path}: the reason above the statement is the answer S2 asks for\n{}",
            run.output()
        );

        repo.write(path, elsewhere);
        let run = repo.weeder(&["check"]);
        assert!(
            run.findings().iter().any(|finding| finding.rule == "S2"),
            "{path}: a comment above another statement explains nothing about this one\n{}",
            run.output()
        );
    }
}
