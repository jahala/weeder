//! weeder's verdict on a pull request, as the one comment the action keeps.
//!
//! The comment is written by the binary, `weeder check --format markdown`, so
//! the order, the words and the links are weeder's own and the same change
//! writes the same bytes. It is read back here the way GitHub reads it, through
//! a CommonMark parser, and held to the SARIF log the same binary writes for the
//! same change: every result, blocks first, each with its rule, its level, its
//! message and a link to the line it names.
//!
//! The action around it is `action.yml`, and it is run here the way a runner
//! runs a composite action: its own steps, in order, handed to bash with the
//! environment GitHub gives them, in a real repository whose pull request
//! deletes a test. The weeder it installs is the one this suite built, packaged
//! by the release's own script and fetched by the action's own install, digest
//! and all. The one step not run here is the one that talks to GitHub; that half
//! is proved on a real pull request by `scripts/check/action-comment.sh`.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde_json::Value;
use weeder::core::catalogue;
use weeder::core::comment::{self, Page};
use weeder::core::review::Approval;
use weeder::core::{Finding, Level, Message, Region, Suppression, SuppressionSource};

use common::{fixture, Repo, Run};

/// Where the links point: a server, a repository and the head commit, as the
/// action spells them for GitHub.
const LINKS: &str =
    "https://github.com/someone/somewhere/blob/0123456789abcdef0123456789abcdef01234567";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The comment and the log for one change, from two runs of the same binary.
fn both(repo: &Repo) -> (Run, Run) {
    let comment = repo.weeder(&["check", "--format", "markdown", "--link-base", LINKS]);
    let log = repo.weeder(&["check", "--format", "sarif"]);
    (comment, log)
}

/// A change with a block, a warning and a note in it: a test file deleted, a
/// workflow added, and a file in a language weeder carries no rules for.
fn mixed() -> Repo {
    let repo = fixture("T1", "rs", "fire");
    repo.write(
        ".github/workflows/build.yml",
        "name: build\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo built\n",
    );
    repo.write("tools/release.rb", "puts 'released'\n");
    repo.stage_all();
    repo
}

// --- the renderer ------------------------------------------------------------

#[test]
fn a_change_with_nothing_to_report_says_so_in_one_sentence() {
    let repo = fixture("T1", "rs", "silent");
    let (comment, log) = both(&repo);

    assert_eq!(comment.code, 0, "{}", comment.stderr);
    assert!(
        log.findings().is_empty(),
        "the fixture should be clean, and it found {:?}",
        log.findings()
    );
    let body = Body::read(&comment.stdout);
    assert!(
        body.items.is_empty(),
        "nothing found is no list:\n{}",
        comment.stdout
    );
    let said = body
        .paragraphs
        .first()
        .map(String::as_str)
        .unwrap_or_default()
        .trim();
    assert!(
        !said.is_empty() && said.ends_with('.') && said.matches(". ").count() == 0,
        "nothing found is said in one sentence, and this is `{said}`"
    );
    assert!(
        said.contains("nothing to report"),
        "the sentence says plainly that there is nothing to report: `{said}`"
    );
}

#[test]
fn every_result_is_listed_blocks_first_with_its_rule_level_message_and_link() {
    let (comment, log) = both(&mixed());

    assert_eq!(
        comment.code, 2,
        "a deleted test blocks:\n{}",
        comment.stderr
    );
    assert_eq!(comment.code, log.code, "both formats reach one verdict");
    let results = ordered(&log.log());
    let levels: Vec<&str> = results
        .iter()
        .map(|result| result["level"].as_str().unwrap_or_default())
        .collect();
    for level in ["error", "warning", "note"] {
        assert!(
            levels.contains(&level),
            "the change should carry a result at {level}, and the levels were {levels:?}"
        );
    }

    let body = Body::read(&comment.stdout);
    assert_eq!(
        body.items.len(),
        results.len(),
        "one item per result:\n{}",
        comment.stdout
    );
    for (item, result) in body.items.iter().zip(&results) {
        let rule = result["ruleId"].as_str().unwrap_or_default();
        let level = result["level"].as_str().unwrap_or_default();
        let message = result["message"]["text"].as_str().unwrap_or_default();
        assert!(
            item.codes.iter().any(|code| code == rule),
            "the item names its rule `{rule}` as code: {item:?}"
        );
        assert!(
            item.lines
                .first()
                .is_some_and(|headline| headline.contains(level)),
            "the item names its level `{level}`: {item:?}"
        );
        assert!(
            folded(&item.text).contains(&folded(message)),
            "the item carries the message as the reader sees it.\nitem:    {}\nmessage: {message}",
            item.text
        );
        let (path, line, end) = place(result);
        let shown = match line {
            Some(line) => format!("{path}:{line}"),
            None => path.clone(),
        };
        // A note on the change as a whole names the repository, `.`, where a
        // blob address has no file to show, so it is shown and links nowhere.
        if path == "." {
            assert!(
                item.places().is_empty(),
                "the repository is no file to link: {item:?}"
            );
            assert!(
                item.codes.contains(&shown),
                "and is shown as written: {item:?}"
            );
            continue;
        }
        assert_eq!(
            item.places().first().map(|link| link.0.as_str()),
            Some(expected_link(&path, line, end).as_str()),
            "the item links to the line it names: {item:?}"
        );
        assert_eq!(
            item.places().first().map(|link| link.1.as_str()),
            Some(shown.as_str()),
            "the link reads as the place it names: {item:?}"
        );
    }
}

#[test]
fn a_path_markdown_would_read_is_linked_and_shown_as_written() {
    let repo = Repo::init();
    let odd = "tests/odd name [1]_#&.rs";
    repo.write(
        odd,
        "#[test]\nfn adds() {\n    assert_eq!(1 + 1, 2);\n}\n\n#[test]\nfn subtracts() {\n    assert_eq!(2 - 1, 1);\n}\n",
    );
    repo.commit("a suite with an awkward name");
    repo.remove(odd);
    repo.stage_all();

    let (comment, log) = both(&repo);
    let results = ordered(&log.log());
    assert!(
        results.iter().any(|result| place(result).0 == odd),
        "the deletion should be reported on `{odd}`: {results:?}"
    );
    let body = Body::read(&comment.stdout);
    assert_eq!(
        body.items.len(),
        results.len(),
        "one item per result:\n{}",
        comment.stdout
    );
    for (item, result) in body.items.iter().zip(&results) {
        let (path, line, end) = place(result);
        let link = item
            .places()
            .first()
            .cloned()
            .expect("every item carries its link");
        assert_eq!(link.0, expected_link(&path, line, end));
        assert!(
            link.1.starts_with(&path),
            "the link shows `{path}` as written, and shows `{}`",
            link.1
        );
    }
}

#[test]
fn the_body_opens_with_a_hidden_marker_naming_weeder() {
    for repo in [fixture("T1", "rs", "silent"), mixed()] {
        let (comment, _) = both(&repo);
        let first = comment.stdout.lines().next().unwrap_or_default();
        assert!(
            first.starts_with("<!--") && first.ends_with("-->") && first.contains("weeder"),
            "the first line is the marker the action finds its comment by, and it is `{first}`"
        );
        let body = Body::read(&comment.stdout);
        assert!(
            body.hidden.iter().any(|html| html.trim() == first),
            "the marker is html a reader never sees: {:?}",
            body.hidden
        );
    }
}

#[test]
fn the_same_change_writes_the_same_bytes_in_every_locale_and_time_zone() {
    let repo = mixed();
    let arguments = ["check", "--format", "markdown", "--link-base", LINKS];
    let runs: Vec<Run> = [
        ("C", "UTC"),
        ("en_US.UTF-8", "Asia/Kathmandu"),
        ("tr_TR.UTF-8", "Pacific/Chatham"),
    ]
    .iter()
    .map(|(locale, zone)| {
        repo.weeder_with(
            &arguments,
            &[("LANG", locale), ("LC_ALL", locale), ("TZ", zone)],
        )
    })
    .collect();
    assert_eq!(runs[0].code, 2, "the change blocks: {}", runs[0].stderr);
    assert!(
        Body::read(&runs[0].stdout).items.len() >= 3,
        "the runs compared a comment with findings in it:\n{}",
        runs[0].stdout
    );
    for run in &runs[1..] {
        assert_eq!(
            run.stdout, runs[0].stdout,
            "the same change, the same bytes"
        );
        assert_eq!(run.code, runs[0].code);
    }
}

#[test]
fn a_run_that_could_not_judge_says_why_in_the_comment() {
    let repo = fixture("T1", "rs", "silent");
    let run = repo.weeder(&[
        "check",
        "--base",
        "no-such-ref",
        "--format",
        "markdown",
        "--link-base",
        LINKS,
    ]);

    assert_eq!(run.code, 3, "{}", run.stderr);
    let first = run.stdout.lines().next().unwrap_or_default();
    assert!(first.starts_with("<!--") && first.contains("weeder"));
    let body = Body::read(&run.stdout);
    let reason = run.stderr.trim();
    assert!(
        !reason.is_empty() && unticked(&body.prose).contains(&unticked(reason)),
        "the comment says why weeder could not judge.\nbody:   {}\nreason: {reason}",
        body.prose
    );
    assert!(
        body.prose.contains("could not judge"),
        "and says it judged nothing: {}",
        body.prose
    );
}

#[test]
fn links_are_refused_where_nothing_would_carry_them() {
    let repo = fixture("T1", "rs", "silent");
    let run = repo.weeder(&["check", "--format", "sarif", "--link-base", LINKS]);

    assert_eq!(run.code, 3, "a flag that would do nothing is refused");
    assert!(
        run.stderr.contains("--link-base") && run.stderr.contains("markdown"),
        "the refusal names the flag and the format it is for: {}",
        run.stderr
    );
}

/// What a finding's own words carry is written so a markdown reader shows it
/// as it was written: a placeholder in angle brackets is not html, a word with
/// underscores is not emphasis, a backtick nobody closed is not code, and a
/// name after an `@` notifies nobody.
#[test]
fn a_message_markdown_would_misread_reads_as_it_was_written() {
    let what = "a test named <RULE> and __init__ and *star* went, with `odd` and a lone ` and @someone on [a] line & more.";
    let finding = Finding {
        rule: "T1".to_string(),
        level: Level::Block,
        path: "tests/a.rs".to_string(),
        region: Some(Region {
            start_line: 3,
            end_line: 3,
        }),
        message: Message {
            what: what.to_string(),
            why: "the suite asks less.".to_string(),
            next: "restore it.".to_string(),
        },
        fix: None,
        suppressed: None,
    };
    let body = Body::read(&comment::render(&[finding], &page(None)));

    let item = body.items.first().expect("the finding is listed");
    let expected = format!("{what} the suite asks less. restore it.");
    assert!(
        folded(&item.text).contains(&folded(&expected)),
        "the message reads as written.\nitem:     {}\nexpected: {expected}",
        item.text
    );
    assert!(
        item.codes.iter().any(|code| code == "odd"),
        "a closed code span stays code: {item:?}"
    );
    assert!(
        item.plain.iter().all(|text| !text.contains("@someone")),
        "a name after an @ sits in code, where GitHub mentions nobody: {item:?}"
    );
    assert!(
        body.hidden.len() == 1,
        "the only html is the marker: {:?}",
        body.hidden
    );
}

/// The page every core-level render here writes: this version, this
/// repository's home, links to the head commit and the command of a run
/// against `main`.
fn page(approval: Option<&Approval>) -> Page<'_> {
    Page {
        version: VERSION,
        home: HOME,
        links: Some(LINKS),
        command: "weeder check --base origin/main --strict",
        approval,
    }
}

/// The version and the home the binary names, as the comment links them.
const VERSION: &str = env!("CARGO_PKG_VERSION");
const HOME: &str = "https://github.com/jahala/weeder";

fn finding(rule: &str, level: Level, path: &str, line: u32, parts: [&str; 3]) -> Finding {
    Finding {
        rule: rule.to_string(),
        level,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: parts[0].to_string(),
            why: parts[1].to_string(),
            next: parts[2].to_string(),
        },
        fix: None,
        suppressed: None,
    }
}

/// Where a rule is documented, for the version that judged.
fn rule_section(rule: &str) -> String {
    format!("{HOME}/blob/v{VERSION}/docs/rules.md#{rule}")
}

/// A URL's query as its pairs, each value percent-decoded.
fn query(url: &str) -> (String, BTreeMap<String, String>) {
    let (address, query) = url.split_once('?').unwrap_or((url, ""));
    let pairs = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key.to_string(), decoded(value))
        })
        .collect();
    (address.to_string(), pairs)
}

fn decoded(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).expect("ascii hex");
            out.push(u8::from_str_radix(hex, 16).expect("a percent escape"));
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).expect("utf-8")
}

/// Each finding opens with a headline: what its rule is about, the level it
/// reports at, and its id linked to its section in `docs/rules.md`, at the
/// version that judged.
#[test]
fn each_finding_opens_with_a_headline_naming_its_rule_and_level() {
    let (comment, log) = both(&mixed());
    let results = ordered(&log.log());
    let body = Body::read(&comment.stdout);
    assert_eq!(body.items.len(), results.len(), "{}", comment.stdout);
    let mut judged = 0;
    for (item, result) in body.items.iter().zip(&results) {
        let rule = result["ruleId"].as_str().unwrap_or_default();
        let level = result["level"].as_str().unwrap_or_default();
        let headline = item.lines.first().expect("an item has a headline");
        assert!(
            headline.contains(level),
            "the headline names the level `{level}`: {headline}"
        );
        let Some(described) = catalogue::rules().iter().find(|known| known.id == rule) else {
            assert!(
                headline.contains(rule),
                "a note about the run names its id `{rule}`: {headline}"
            );
            continue;
        };
        judged += 1;
        assert!(
            item.strong
                .first()
                .is_some_and(|strong| strong == described.short_description),
            "the headline opens with the rule's short description in bold, `{}`: {item:?}",
            described.short_description
        );
        assert!(
            item.links
                .iter()
                .any(|(url, shown)| url == &rule_section(rule) && shown == rule),
            "the rule id links to its section, {}: {item:?}",
            rule_section(rule)
        );
    }
    assert!(judged >= 2, "the change was judged by at least two rules");
}

/// What was found, why it matters and the next action stand on lines of their
/// own, each opening with a capital, rendered from the finding's own three
/// parts. weeder's own name stays lowercase wherever it stands, as its voice
/// says.
#[test]
fn what_why_and_next_stand_on_lines_of_their_own_each_opening_with_a_capital() {
    let findings = [
        finding(
            "T1",
            Level::Block,
            "tests/a.rs",
            3,
            [
                "a test file was deleted.",
                "a suite that is gone reports nothing.",
                "restore the file.",
            ],
        ),
        finding(
            "C3",
            Level::Warn,
            ".github/workflows/build.yml",
            1,
            [
                "weeder saw a workflow change.",
                "`cargo test` decides less now.",
                "read it.",
            ],
        ),
    ];
    let body = Body::read(&comment::render(&findings, &page(None)));
    assert_eq!(body.items.len(), 2);
    for (item, expected) in body.items.iter().zip([
        [
            "A test file was deleted.",
            "A suite that is gone reports nothing.",
            "Restore the file.",
        ],
        [
            "weeder saw a workflow change.",
            "cargo test decides less now.",
            "Read it.",
        ],
    ]) {
        let at = item
            .lines
            .iter()
            .position(|line| line == expected[0])
            .unwrap_or_else(|| panic!("`{}` stands on a line of its own: {item:?}", expected[0]));
        assert_eq!(
            &item.lines[at..at + 3],
            &expected,
            "what, why and next follow one another, a line each: {item:?}"
        );
    }

    // The same reading of a finding's parts from the binary: every line of
    // what, why and next opens with a capital or with weeder's own name.
    let (comment, _) = both(&mixed());
    let body = Body::read(&comment.stdout);
    for item in &body.items {
        assert!(
            item.lines.len() >= 5,
            "a headline, a place and three parts: {item:?}"
        );
        for line in &item.lines[2..5] {
            assert!(
                !line.starts_with(|first: char| first.is_lowercase())
                    || line.starts_with("weeder "),
                "`{line}` opens with a capital, or a number: {item:?}"
            );
        }
    }
}

/// Every judgement carries a way to say it is wrong: a link that opens a new
/// issue on weeder's own repository from the false-positive template, naming
/// the rule and the version and nothing else, because the change judged may
/// sit in a private repository and the tracker is public.
#[test]
fn every_judgement_links_to_a_false_positive_report_naming_only_the_rule_and_the_version() {
    let (comment, log) = both(&mixed());
    let results = ordered(&log.log());
    let body = Body::read(&comment.stdout);
    let mut reported = 0;
    for (item, result) in body.items.iter().zip(&results) {
        let rule = result["ruleId"].as_str().unwrap_or_default();
        let reports: Vec<&(String, String)> = item
            .links
            .iter()
            .filter(|(url, _)| url.starts_with(&format!("{HOME}/issues/new")))
            .collect();
        if !catalogue::rules().iter().any(|known| known.id == rule) {
            assert!(
                reports.is_empty(),
                "a note about the run is no judgement to report: {item:?}"
            );
            continue;
        }
        reported += 1;
        let [(url, shown)] = reports.as_slice() else {
            panic!("one report link per judgement: {item:?}");
        };
        assert_eq!(
            shown, "Wrong? Report it",
            "the link says what it is for: {item:?}"
        );
        let (address, pairs) = query(url);
        assert_eq!(address, format!("{HOME}/issues/new"));
        let keys: Vec<&str> = pairs.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            ["rule", "template", "title", "version"],
            "the link carries the template, a title, the rule and the version, and nothing else: {url}"
        );
        assert_eq!(pairs["template"], "false-positive.yml");
        assert_eq!(pairs["rule"], rule);
        assert_eq!(pairs["version"], VERSION);
        let (path, _, _) = place(result);
        let file = path.rsplit('/').next().unwrap_or_default().to_string();
        for (key, value) in &pairs {
            assert!(
                !value.contains(&path) && !value.contains(&file),
                "`{key}` carries nothing from the judged repository: {url}"
            );
        }
        assert!(
            pairs["title"].contains(rule),
            "the title names the rule: {}",
            pairs["title"]
        );
    }
    assert!(reported >= 2, "the change was judged by at least two rules");

    let template = yaml(&root().join(".github/ISSUE_TEMPLATE/false-positive.yml"));
    let fields: Vec<&str> = template["body"]
        .as_array()
        .expect("the template is an issue form with a body")
        .iter()
        .filter_map(|field| field["id"].as_str())
        .collect();
    for wanted in ["rule", "version"] {
        assert!(
            fields.contains(&wanted),
            "the template has a field `{wanted}` the link fills in: {fields:?}"
        );
    }
    assert!(
        template["name"].is_string() && template["description"].is_string(),
        "the template names itself in the chooser: {template}"
    );
}

/// The comment ends with the command that judges the same change the same way
/// on a person's own machine, and running that command gives the same verdict.
#[test]
fn the_comment_ends_with_the_command_that_reproduces_the_run() {
    let repo = mixed();
    repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    repo.git(&["switch", "--quiet", "-c", "feature"]);
    repo.commit("the change");
    let comment = repo.weeder(&[
        "check",
        "--base",
        "origin/main",
        "--strict",
        "--format",
        "markdown",
        "--link-base",
        LINKS,
    ]);
    assert_eq!(comment.code, 2, "{}", comment.stderr);
    let body = Body::read(&comment.stdout);
    let last = body.paragraphs.last().expect("the comment ends with prose");
    assert!(
        last.contains("weeder check --base origin/main --strict"),
        "the last paragraph names the command: {last}"
    );
    let tail = comment.stdout.trim_end().lines().last().unwrap_or_default();
    assert!(
        tail.contains("`weeder check --base origin/main --strict`"),
        "and the command, as code, is the last thing in the body: {tail}"
    );

    let again = repo.weeder(&[
        "check",
        "--base",
        "origin/main",
        "--strict",
        "--format",
        "sarif",
    ]);
    assert_eq!(
        again.code, comment.code,
        "the command reproduces the verdict"
    );

    let plain = repo.weeder(&["check", "--format", "markdown"]);
    let last = Body::read(&plain.stdout)
        .paragraphs
        .last()
        .cloned()
        .unwrap_or_default();
    assert!(
        last.contains("weeder check") && !last.contains("--base"),
        "a run with no flags names the command with none: {last}"
    );
}

/// The same findings write the same bytes, in whatever order a face gathered
/// them.
#[test]
fn the_same_findings_write_the_same_bytes_in_any_order() {
    let findings = vec![
        finding(
            "T1",
            Level::Block,
            "tests/b.rs",
            4,
            ["b went.", "less.", "restore it."],
        ),
        finding(
            "T1",
            Level::Block,
            "tests/a.rs",
            9,
            ["a went.", "less.", "restore it."],
        ),
        finding(
            "C3",
            Level::Warn,
            ".github/workflows/x.yml",
            1,
            ["a workflow.", "it runs.", "read it."],
        ),
        finding(
            "T1",
            Level::Block,
            "tests/a.rs",
            9,
            ["a went too.", "less.", "restore it."],
        ),
    ];
    let mut reversed = findings.clone();
    reversed.reverse();
    let forwards = comment::render(&findings, &page(None));
    assert_eq!(forwards, comment::render(&reversed, &page(None)));
    assert_eq!(forwards, comment::render(&findings, &page(None)));
}

// --- two keys for a false positive -------------------------------------------

/// GitHub's own answers, recorded from this repository: the reviews of a pull
/// request, and the permission of an owner, of an account with no access, and
/// of a bot.
fn recorded(name: &str) -> Value {
    let path = root().join("fixtures/github").join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} should read: {error}", path.display()));
    serde_json::from_str(&text).expect("a recorded answer is json")
}

/// One review as GitHub answers it, given by the account whose permission
/// answer is named, in the state and at the commit asked for.
fn review(id: u64, state: &str, commit: &str, permission: &str) -> Value {
    let mut review = recorded("reviews.json")[0].clone();
    review["id"] = Value::from(id);
    review["state"] = Value::from(state);
    review["commit_id"] = Value::from(commit);
    review["user"] = recorded(permission)["user"].clone();
    review
}

/// What the action hands weeder: the head commit, the reviews, and the
/// permission of every account that reviewed, each as GitHub answered.
struct Reviews {
    _scratch: tempfile::TempDir,
    path: PathBuf,
}

fn reviews(head: &str, given: &[Value], permissions: &[&str]) -> Reviews {
    let scratch = tempfile::TempDir::new().expect("a scratch directory");
    let path = scratch.path().join("reviews.json");
    let answers = serde_json::json!({
        "head": head,
        "reviews": given,
        "permissions": permissions.iter().map(|name| recorded(name)).collect::<Vec<Value>>(),
    });
    std::fs::write(&path, answers.to_string()).expect("the answers write");
    Reviews {
        _scratch: scratch,
        path,
    }
}

const ALLOWED: &str =
    "Drop the old suite\n\nWeeder-allow: T1 the cases moved to the integration tier\n";

/// A pull request onto `main` whose one commit deletes a test file, with the
/// message the caller writes on it.
fn deleting_a_suite(message: &str) -> Repo {
    let repo = Repo::init();
    repo.write(
        "src/lib.rs",
        "pub fn double(x: i32) -> i32 {\n    x * 2\n}\n",
    );
    repo.write(
        "tests/double.rs",
        "use demo::double;\n\n#[test]\nfn doubles() {\n    assert_eq!(double(2), 4);\n}\n\n#[test]\nfn doubles_zero() {\n    assert_eq!(double(0), 0);\n}\n",
    );
    repo.commit("the suite begins");
    repo.git(&["switch", "--quiet", "-c", "feature"]);
    repo.remove("tests/double.rs");
    repo.commit(message);
    repo
}

fn judged(repo: &Repo, format: &str, answers: Option<&Reviews>) -> Run {
    let mut arguments = vec!["check", "--base", "main", "--strict", "--format", format];
    if format == "markdown" {
        arguments.extend(["--link-base", LINKS]);
    }
    let path;
    if let Some(answers) = answers {
        path = answers.path.display().to_string();
        arguments.extend(["--reviews", path.as_str()]);
    }
    repo.weeder(&arguments)
}

fn item_for(body: &Body, rule: &str) -> Item {
    body.items
        .iter()
        .find(|item| item.codes.iter().any(|code| code == rule))
        .cloned()
        .unwrap_or_else(|| panic!("the comment lists {rule}: {body:?}"))
}

/// The second key: a trailer with a reason is accepted once someone with write
/// access approves the pull request at its head, and the comment says by whom
/// and at which commit.
#[test]
fn a_trailer_approved_at_the_head_by_someone_with_write_access_is_accepted() {
    let repo = deleting_a_suite(ALLOWED);
    let head = repo.head();
    let answers = reviews(
        &head,
        &[
            recorded("reviews.json")[0].clone(),
            review(2, "APPROVED", &head, "permission-admin.json"),
        ],
        &["permission-admin.json", "permission-none.json"],
    );

    let comment = judged(&repo, "markdown", Some(&answers));
    assert_eq!(
        comment.code, 0,
        "the accepted block lets the change through:\n{}{}",
        comment.stdout, comment.stderr
    );
    let log = judged(&repo, "sarif", Some(&answers));
    assert_eq!(
        log.code, 0,
        "both formats reach one verdict: {}",
        log.stderr
    );

    let body = Body::read(&comment.stdout);
    let item = item_for(&body, "T1");
    assert!(
        item.lines[0].contains("accepted"),
        "the headline says the finding was accepted: {item:?}"
    );
    let said = item
        .lines
        .iter()
        .find(|line| line.starts_with("Accepted by"))
        .unwrap_or_else(|| panic!("a line says who accepted it: {item:?}"));
    assert!(
        said.contains("jahala") && said.contains(&head),
        "it names the reviewer and the commit: {said}"
    );
    assert!(
        item.codes.iter().any(|code| code == "jahala"),
        "the reviewer is named in code, where GitHub notifies nobody: {item:?}"
    );
    assert!(
        said.contains("the cases moved to the integration tier"),
        "and the reason the trailer gave: {said}"
    );
}

/// GitHub checks a pull request out as the merge of its head into its base, so
/// the head the reviewer approved is the second parent of what was judged.
#[test]
fn an_approval_of_the_head_a_merge_checkout_carries_is_accepted() {
    let repo = deleting_a_suite(ALLOWED);
    let head = repo.head();
    repo.git(&["switch", "--quiet", "--detach", "main"]);
    repo.git(&[
        "merge",
        "--quiet",
        "--no-ff",
        "-m",
        "Merge the pull request",
        &head,
    ]);
    let answers = reviews(
        &head,
        &[review(2, "APPROVED", &head, "permission-admin.json")],
        &["permission-admin.json"],
    );
    let comment = judged(&repo, "markdown", Some(&answers));
    assert_eq!(comment.code, 0, "{}{}", comment.stdout, comment.stderr);
}

/// Everything else leaves the finding blocking.
#[test]
fn an_older_commit_a_reader_a_bot_a_withdrawn_approval_and_another_head_all_leave_it_blocking() {
    let repo = deleting_a_suite(ALLOWED);
    let head = repo.head();
    let older = repo.git(&["rev-parse", "main"]).trim().to_string();
    let admin = "permission-admin.json";
    let cases: Vec<(&str, Reviews)> = vec![
        (
            "a review of an older commit",
            reviews(&head, &[review(2, "APPROVED", &older, admin)], &[admin]),
        ),
        (
            "a review from someone without write access",
            reviews(
                &head,
                &[review(2, "APPROVED", &head, "permission-read.json")],
                &["permission-read.json"],
            ),
        ),
        (
            "a review from a bot",
            reviews(
                &head,
                &[review(2, "APPROVED", &head, "permission-none.json")],
                &["permission-none.json"],
            ),
        ),
        (
            "an approval its reviewer then asked changes on",
            reviews(
                &head,
                &[
                    review(2, "APPROVED", &head, admin),
                    review(3, "CHANGES_REQUESTED", &head, admin),
                ],
                &[admin],
            ),
        ),
        (
            "an approval that was dismissed",
            reviews(&head, &[review(2, "DISMISSED", &head, admin)], &[admin]),
        ),
        (
            "an approval whose reviewer's permission was never read",
            reviews(&head, &[review(2, "APPROVED", &head, admin)], &[]),
        ),
        (
            "answers about another head than the one judged",
            reviews(&older, &[review(2, "APPROVED", &older, admin)], &[admin]),
        ),
    ];
    for (case, answers) in &cases {
        let comment = judged(&repo, "markdown", Some(answers));
        assert_eq!(
            comment.code, 2,
            "{case} leaves the block standing:\n{}{}",
            comment.stdout, comment.stderr
        );
        let item = item_for(&Body::read(&comment.stdout), "T1");
        assert!(
            !item.lines[0].contains("accepted"),
            "{case}: nothing was accepted: {item:?}"
        );
    }
    // A comment after an approval leaves the approval standing.
    let commented = reviews(
        &head,
        &[
            review(2, "APPROVED", &head, admin),
            review(3, "COMMENTED", &head, admin),
        ],
        &[admin],
    );
    assert_eq!(judged(&repo, "markdown", Some(&commented)).code, 0);
}

#[test]
fn a_trailer_with_no_reason_and_an_inline_marker_stay_blocking_under_an_approval() {
    let admin = "permission-admin.json";
    let bare = deleting_a_suite("Drop the old suite\n\nWeeder-allow: T1\n");
    let head = bare.head();
    let approved = reviews(&head, &[review(2, "APPROVED", &head, admin)], &[admin]);
    let comment = judged(&bare, "markdown", Some(&approved));
    assert_eq!(
        comment.code, 2,
        "a trailer with no reason allows nothing:\n{}",
        comment.stdout
    );

    let repo = Repo::init();
    repo.write(
        "src/lib.rs",
        "pub fn double(x: i32) -> i32 {\n    x * 2\n}\n",
    );
    repo.write(
        "tests/double.rs",
        "use demo::double;\n\n#[test]\nfn doubles() {\n    assert_eq!(double(2), 4);\n}\n",
    );
    repo.commit("the suite begins");
    repo.git(&["switch", "--quiet", "-c", "feature"]);
    repo.write(
        "tests/double.rs",
        "use demo::double;\n\n#[test]\n#[ignore] // weeder-allow T3: the runner is down this week\nfn doubles() {\n    assert_eq!(double(2), 4);\n}\n",
    );
    repo.commit("Skip the doubling test");
    let head = repo.head();
    let approved = reviews(&head, &[review(2, "APPROVED", &head, admin)], &[admin]);
    let comment = judged(&repo, "markdown", Some(&approved));
    assert_eq!(
        comment.code, 2,
        "an inline marker is never accepted:\n{}",
        comment.stdout
    );
    let item = item_for(&Body::read(&comment.stdout), "T3");
    assert!(!item.lines[0].contains("accepted"), "{item:?}");
}

/// A block that still stands says how to let it through on the record: the
/// reason in a trailer, and a reviewer with write access to approve.
#[test]
fn a_blocking_finding_says_to_write_the_reason_and_ask_a_reviewer_with_write_access() {
    let repo = deleting_a_suite("Drop the old suite\n");
    let comment = judged(&repo, "markdown", None);
    assert_eq!(comment.code, 2);
    let item = item_for(&Body::read(&comment.stdout), "T1");
    let line = item
        .lines
        .iter()
        .find(|line| line.contains("Weeder-allow: T1 <reason>"))
        .unwrap_or_else(|| panic!("a line names the trailer: {item:?}"));
    assert!(
        line.contains("write access") && line.contains("approve"),
        "and asks for a reviewer with write access to approve: {line}"
    );

    // The same finding with its trailer written says what is still missing.
    let repo = deleting_a_suite(ALLOWED);
    let comment = judged(&repo, "markdown", None);
    assert_eq!(comment.code, 2);
    let item = item_for(&Body::read(&comment.stdout), "T1");
    assert!(
        item.lines.iter().any(
            |line| line.contains("the cases moved to the integration tier")
                && line.contains("write access")
        ),
        "the trailer's reason is shown, waiting on an approval: {item:?}"
    );
}

#[test]
fn reviews_are_refused_where_they_could_accept_nothing() {
    let repo = deleting_a_suite(ALLOWED);
    let head = repo.head();
    let admin = "permission-admin.json";
    let answers = reviews(&head, &[review(2, "APPROVED", &head, admin)], &[admin]);
    let path = answers.path.display().to_string();
    for (arguments, named) in [
        (
            vec!["check", "--base", "main", "--reviews", path.as_str()],
            "--strict",
        ),
        (
            vec!["check", "--strict", "--reviews", path.as_str()],
            "--base",
        ),
    ] {
        let run = repo.weeder(&arguments);
        assert_eq!(run.code, 3, "{arguments:?} is refused: {}", run.stderr);
        assert!(
            run.stderr.contains("--reviews") && run.stderr.contains(named),
            "the refusal names what is missing: {}",
            run.stderr
        );
    }
    let unreadable = tempfile::TempDir::new().expect("scratch");
    let broken = unreadable.path().join("reviews.json");
    std::fs::write(&broken, "{ not json").expect("writes");
    let run = repo.weeder(&[
        "check",
        "--base",
        "main",
        "--strict",
        "--reviews",
        &broken.display().to_string(),
    ]);
    assert_eq!(
        run.code, 3,
        "answers weeder cannot read are a run that never happened: {}",
        run.stderr
    );
}

/// The accepted finding as core renders it, from the findings and the approval
/// the face hands in.
#[test]
fn an_accepted_finding_names_its_reviewers_and_commit() {
    let mut accepted = finding(
        "T1",
        Level::Note,
        "tests/a.rs",
        3,
        ["a went.", "less.", "restore it."],
    );
    accepted.suppressed = Some(Suppression {
        rule: "T1".to_string(),
        reason: "it moved to _the_ other tier".to_string(),
        path: None,
        line: None,
        source: SuppressionSource::CommitTrailer,
        original_level: Some(Level::Block),
    });
    let approval = Approval {
        reviewers: vec!["another".to_string(), "one-reviewer".to_string()],
        commit: "0123456789abcdef0123456789abcdef01234567".to_string(),
    };
    let body = Body::read(&comment::render(&[accepted], &page(Some(&approval))));
    let item = body.items.first().expect("listed");
    let line = item
        .lines
        .iter()
        .find(|line| line.starts_with("Accepted by"))
        .unwrap_or_else(|| panic!("{item:?}"));
    assert!(
        line.contains("one-reviewer") && line.contains("another"),
        "{line}"
    );
    assert!(line.contains(&approval.commit), "{line}");
    assert!(
        line.contains("it moved to _the_ other tier"),
        "the reason reads as written: {line}"
    );
    assert!(
        body.paragraphs
            .first()
            .is_some_and(|said| said.contains("accepted")),
        "the opening sentence counts it as accepted: {:?}",
        body.paragraphs
    );
}

// --- the action --------------------------------------------------------------

fn yaml(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} should read: {error}", path.display()));
    serde_yaml_ng::from_str(&text)
        .unwrap_or_else(|error| panic!("{} should be yaml: {error}", path.display()))
}

fn action() -> Value {
    yaml(&root().join("action.yml"))
}

fn example() -> Value {
    yaml(&root().join("examples/ci/github.yml"))
}

fn cargo_version() -> String {
    let manifest = std::fs::read_to_string(root().join("Cargo.toml")).expect("Cargo.toml reads");
    manifest
        .lines()
        .find_map(|line| line.strip_prefix("version = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("Cargo.toml names a version")
        .to_string()
}

#[test]
fn the_example_asks_for_what_the_comment_and_the_upload_need_and_nothing_else() {
    let workflow = example();
    let jobs = workflow["jobs"].as_object().expect("the example has jobs");
    assert_eq!(jobs.len(), 1, "the example is one job a reader copies");
    let job = jobs.values().next().expect("one job");

    let granted: BTreeMap<String, String> = job["permissions"]
        .as_object()
        .expect("the job states its permissions")
        .iter()
        .map(|(key, value)| (key.clone(), value.as_str().unwrap_or_default().to_string()))
        .collect();
    let wanted: BTreeMap<String, String> = [
        ("contents", "read"),
        ("pull-requests", "write"),
        ("security-events", "write"),
    ]
    .iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect();
    assert_eq!(
        granted, wanted,
        "the job asks for exactly what the comment and the upload need"
    );
    let above = workflow["permissions"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    for (key, value) in &above {
        assert!(
            key == "contents" && value == "read",
            "the workflow grants nothing above the job beyond reading: {key}: {value}"
        );
    }

    let uses: Vec<&str> = job["steps"]
        .as_array()
        .expect("the job has steps")
        .iter()
        .filter_map(|step| step["uses"].as_str())
        .collect();
    let tag = format!("jahala/weeder@v{}", cargo_version());
    assert!(
        uses.contains(&tag.as_str()),
        "the example runs the action by the tag of this version, `{tag}`, and uses {uses:?}"
    );
    let runs = job["steps"]
        .as_array()
        .expect("the job has steps")
        .iter()
        .filter(|step| step["run"].is_string())
        .count();
    assert_eq!(
        runs, 0,
        "the example installs and runs weeder through the action alone"
    );
}

fn step_script<'a>(action: &'a Value, id: &str) -> &'a str {
    action["runs"]["steps"]
        .as_array()
        .and_then(|steps| steps.iter().find(|step| step["id"] == id))
        .and_then(|step| step["run"].as_str())
        .unwrap_or_else(|| panic!("the action has a `{id}` step that runs a script"))
}

/// The action reads the pull request's reviews and the permission of everyone
/// who reviewed, over the API with the job's own token, and hands GitHub's
/// answers to weeder, which decides; both judgements read the same answers, so
/// the comment and the log cannot reach two verdicts.
#[test]
fn the_action_reads_the_reviews_and_each_reviewers_permission_and_weeder_decides() {
    let action = action();
    let review = step_script(&action, "review");
    for read in ["pulls/$NUMBER/reviews", "collaborators/", "/permission"] {
        assert!(
            review.contains(read),
            "the review step reads `{read}`:\n{review}"
        );
    }
    let judge = step_script(&action, "judge");
    assert_eq!(
        judge.matches("\"${reviewed[@]}\"").count(),
        2,
        "both judgements are handed the reviews:\n{judge}"
    );
    let env = action["runs"]["steps"]
        .as_array()
        .and_then(|steps| steps.iter().find(|step| step["id"] == "judge"))
        .map(|step| step["env"].clone())
        .unwrap_or_default();
    assert_eq!(env["REVIEWS"], "${{ steps.review.outputs.reviews }}");
    assert_eq!(
        env["BASE_REF"], "${{ github.event.pull_request.base.ref }}",
        "the base is read from the payload, which a review's run carries too"
    );
}

/// An approving review runs the action again, and so does withdrawing one.
#[test]
fn a_review_runs_the_action_again() {
    for path in [
        root().join("examples/ci/github.yml"),
        root().join(".github/workflows/comment.yml"),
    ] {
        let workflow = yaml(&path);
        assert_eq!(
            workflow["on"]["pull_request_review"]["types"],
            serde_json::json!(["submitted", "dismissed"]),
            "{} runs on a review submitted or dismissed",
            path.display()
        );
        assert!(
            workflow["on"].get("pull_request").is_some(),
            "{} still runs on every push",
            path.display()
        );
    }
    let text = std::fs::read_to_string(root().join("action.yml")).expect("action.yml reads");
    assert!(
        text.contains("pull_request_review"),
        "the action documents the review trigger"
    );
}

#[test]
fn nothing_here_runs_on_pull_request_target() {
    let mut workflows = vec![root().join("examples/ci/github.yml")];
    for entry in std::fs::read_dir(root().join(".github/workflows")).expect("workflows read") {
        workflows.push(entry.expect("a workflow entry").path());
    }
    for path in workflows {
        let workflow = yaml(&path);
        let on = &workflow["on"];
        let named = match on {
            Value::String(event) => vec![event.clone()],
            Value::Array(events) => events
                .iter()
                .filter_map(|event| event.as_str().map(str::to_string))
                .collect(),
            Value::Object(events) => events.keys().cloned().collect(),
            _ => Vec::new(),
        };
        assert!(
            !named.iter().any(|event| event == "pull_request_target"),
            "{} runs on pull_request_target, which hands a fork's code a token that can write",
            path.display()
        );
    }
}

/// Code scanning takes the findings where it can (the owner's ruling of
/// 2026-11-13): from a public repository unless the pull request comes out of
/// a fork, whose token cannot upload, and from a private one only when it says
/// it has Advanced Security. The example passes the repository's own variable.
#[test]
fn the_upload_runs_where_code_scanning_can_take_it_and_nowhere_else() {
    let action = action();
    let input = &action["inputs"]["code-scanning"];
    assert_eq!(
        input["default"], "auto",
        "the input defaults to auto: {input}"
    );
    let upload = action["runs"]["steps"]
        .as_array()
        .expect("the action has steps")
        .iter()
        .find(|step| step["id"] == "upload")
        .expect("the action has an upload step");
    let uses = upload["uses"].as_str().unwrap_or_default();
    let (name, pin) = uses.split_once('@').expect("the upload names a version");
    assert_eq!(name, "github/codeql-action/upload-sarif");
    assert!(
        pin.len() == 40 && pin.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "the upload is pinned to a commit, not a tag: {uses}"
    );
    let condition: String = upload["if"]
        .as_str()
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for clause in [
        "inputs.code-scanning == 'on'",
        "inputs.code-scanning == 'auto'",
        "!github.event.repository.private",
        "github.event.pull_request.head.repo.full_name == github.repository",
    ] {
        assert!(
            condition.contains(clause),
            "the upload's condition holds `{clause}`: {condition}"
        );
    }
    assert_eq!(
        upload["with"]["sarif_file"], "${{ steps.judge.outputs.sarif }}",
        "it uploads the log the judge step wrote"
    );

    let example = example();
    let step = example["jobs"]
        .as_object()
        .and_then(|jobs| jobs.values().next())
        .and_then(|job| job["steps"].as_array())
        .and_then(|steps| {
            steps.iter().find(|step| {
                step["uses"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("jahala/weeder@")
            })
        })
        .expect("the example runs the action");
    assert_eq!(
        step["with"]["code-scanning"], "${{ vars.WEEDER_CODE_SCANNING || 'auto' }}",
        "the example lets a repository opt in through its own variable"
    );
}

#[test]
fn the_action_is_steps_a_runner_runs_in_order_with_no_condition_to_skip_one() {
    let action = action();
    assert_eq!(action["runs"]["using"], "composite");
    let steps = action["runs"]["steps"]
        .as_array()
        .expect("the action has steps");
    let ids: Vec<&str> = steps
        .iter()
        .map(|step| step["id"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        ids,
        ["install", "review", "judge", "summary", "comment", "upload", "verdict"],
        "install, read the reviews, judge, then the summary, the comment and the upload, and the verdict last"
    );
    for step in steps {
        assert!(
            step.get("continue-on-error").is_none(),
            "no step is told it may fail: {step}"
        );
        if step["id"] == "upload" {
            continue;
        }
        assert!(
            step.get("if").is_none(),
            "no step but the upload decides by a condition whether it runs: {step}"
        );
        assert_eq!(step["shell"], "bash", "every step is bash: {step}");
        let script = step["run"].as_str().unwrap_or_default();
        assert!(
            !script.contains("${{"),
            "a step reads what GitHub hands it from its env, never spliced into the script: {step}"
        );
    }
}

/// The action's steps, run against a real repository the way a runner runs
/// them. The comment step is the one that talks to GitHub, and it is run here
/// with no token, the way a job with no right to write would run it.
#[cfg(unix)]
mod running {
    use super::*;

    use tempfile::TempDir;

    /// The world one run of the action sees: a runner's temp directory, the
    /// files a step appends its outputs, PATH and summary to, and a release
    /// directory holding the archive this suite built.
    struct Runner {
        scratch: TempDir,
        archive: String,
    }

    impl Runner {
        fn new() -> Runner {
            let scratch = TempDir::new().expect("a scratch directory");
            let release = scratch.path().join("release");
            let binary = common::binary();
            let directory = binary.parent().expect("the binary sits in a directory");
            let target = "this-machine";
            let packaged = common::bash(&root().join("scripts/package-release.sh"))
                .args([target, &directory.display().to_string()])
                .arg(&release)
                .output()
                .expect("the packaging script runs");
            assert!(
                packaged.status.success(),
                "the release's own packaging should package this build: {}",
                String::from_utf8_lossy(&packaged.stderr)
            );
            for name in ["temp", "home", "gh"] {
                std::fs::create_dir_all(scratch.path().join(name)).expect("a runner directory");
            }
            let archive = format!(
                "file://{}",
                release.join(format!("weeder-{target}.tar.gz")).display()
            );
            Runner { scratch, archive }
        }

        fn file(&self, name: &str) -> PathBuf {
            self.scratch.path().join(name)
        }

        fn read(&self, name: &str) -> String {
            std::fs::read_to_string(self.file(name)).unwrap_or_default()
        }
    }

    struct Step {
        id: String,
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// Every `${{ }}` an env block of the action may hold, filled in the way
    /// GitHub fills it for a pull request whose base is `main`.
    fn expressions(
        runner: &Runner,
        archive: &str,
        event: &Event,
        outputs: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let mut known: BTreeMap<String, String> = [
            ("inputs.token", ""),
            ("inputs.archive", archive),
            ("github.action_path", &root().display().to_string()),
            ("github.base_ref", event.base_ref),
            ("github.event_name", event.name),
            ("github.event.pull_request.base.ref", "main"),
            ("github.server_url", "https://github.com"),
            ("github.repository", "someone/somewhere"),
            ("github.event.pull_request.number", "7"),
            (
                "github.event.pull_request.head.sha",
                "0123456789abcdef0123456789abcdef01234567",
            ),
            ("runner.os", host_os()),
            ("runner.arch", host_arch()),
            ("runner.temp", &runner.file("temp").display().to_string()),
        ]
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
        for (key, value) in outputs {
            known.insert(key.clone(), value.clone());
        }
        known
    }

    fn host_os() -> &'static str {
        if cfg!(target_os = "macos") {
            "macOS"
        } else {
            "Linux"
        }
    }

    fn host_arch() -> &'static str {
        if cfg!(target_arch = "aarch64") {
            "ARM64"
        } else {
            "X64"
        }
    }

    fn is_a_step_output(expression: &str) -> bool {
        let words: Vec<&str> = expression.split('.').collect();
        let action = action();
        matches!(words.as_slice(), ["steps", id, "outputs", _]
            if action["runs"]["steps"]
                .as_array()
                .is_some_and(|steps| steps.iter().any(|step| step["id"] == *id)))
    }

    fn resolve(text: &str, known: &BTreeMap<String, String>) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(start) = rest.find("${{") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 3..];
            let end = after.find("}}").expect("an expression closes");
            let expression = after[..end].trim();
            // An output a step of this action never wrote reads as nothing, as
            // GitHub reads it.
            let value = match known.get(expression) {
                Some(value) => value.as_str(),
                None if is_a_step_output(expression) => "",
                None => panic!("the action uses `{expression}`, which this run cannot fill in"),
            };
            out.push_str(value);
            rest = &after[end + 2..];
        }
        out.push_str(rest);
        out
    }

    /// The action's steps run in the repository, each one only if the one
    /// before it passed, the way a composite action runs them.
    /// The event a run was started by, as GitHub names it, and the base ref
    /// its context carries: `github.base_ref` is filled for `pull_request` and
    /// left empty for `pull_request_review`, whose payload still names the
    /// pull request's base.
    struct Event {
        name: &'static str,
        base_ref: &'static str,
    }

    const PULL_REQUEST: Event = Event {
        name: "pull_request",
        base_ref: "main",
    };

    const REVIEW: Event = Event {
        name: "pull_request_review",
        base_ref: "",
    };

    fn run_action(runner: &Runner, repo: &Repo, archive: &str) -> Vec<Step> {
        run_action_on(runner, repo, archive, &PULL_REQUEST)
    }

    fn run_action_on(runner: &Runner, repo: &Repo, archive: &str, event: &Event) -> Vec<Step> {
        // A fixture's pull request is its working tree over HEAD; a history
        // that already names its base keeps it.
        if repo
            .try_git(&[
                "rev-parse",
                "--verify",
                "--quiet",
                "refs/remotes/origin/main",
            ])
            .code
            != 0
        {
            repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        }
        let action = action();
        let mut outputs: BTreeMap<String, String> = BTreeMap::new();
        let mut ran = Vec::new();
        for (at, step) in action["runs"]["steps"]
            .as_array()
            .expect("the action has steps")
            .iter()
            .enumerate()
        {
            let id = step["id"].as_str().unwrap_or_default().to_string();
            // A step that uses another action runs only on GitHub, which this
            // suite never stands in for; the live pull request proves it.
            if step.get("uses").is_some() {
                continue;
            }
            let known = expressions(runner, archive, event, &outputs);
            let script = runner.file(&format!("step-{at}.sh"));
            // A runner hands a step to `bash --noprofile --norc -eo pipefail`;
            // the options are written into the script, so the harness's bash
            // runs it the same way.
            std::fs::write(
                &script,
                format!(
                    "set -eo pipefail\n{}",
                    step["run"].as_str().unwrap_or_default()
                ),
            )
            .expect("the step's script writes");
            let output_file = runner.file(&format!("output-{at}"));
            std::fs::write(&output_file, "").expect("an output file");
            let mut command = common::bash(&script);
            command
                .current_dir(repo.root())
                .env_clear()
                .env("PATH", path(runner))
                .env("HOME", runner.file("home"))
                .env("GH_CONFIG_DIR", runner.file("gh"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GITHUB_OUTPUT", &output_file)
                .env("GITHUB_PATH", runner.file("github-path"))
                .env("GITHUB_STEP_SUMMARY", runner.file("summary.md"))
                .env("RUNNER_TEMP", runner.file("temp"));
            if let Some(env) = step["env"].as_object() {
                for (key, value) in env {
                    command.env(key, resolve(value.as_str().unwrap_or_default(), &known));
                }
            }
            let output = command.output().expect("bash runs the step");
            for line in std::fs::read_to_string(&output_file)
                .unwrap_or_default()
                .lines()
            {
                if let Some((key, value)) = line.split_once('=') {
                    outputs.insert(format!("steps.{id}.outputs.{key}"), value.to_string());
                }
            }
            let code = output.status.code().unwrap_or(-1);
            ran.push(Step {
                id,
                code,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
            if code != 0 {
                break;
            }
        }
        ran
    }

    /// The system's own programs and nothing of this suite's: the weeder that
    /// judges is the one the action installed.
    fn path(runner: &Runner) -> String {
        let added: Vec<String> = runner
            .read("github-path")
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .rev()
            .collect();
        let mut entries = added;
        entries.push("/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin:/usr/local/bin".to_string());
        entries.join(":")
    }

    fn step<'a>(ran: &'a [Step], id: &str) -> &'a Step {
        ran.iter()
            .find(|step| step.id == id)
            .unwrap_or_else(|| panic!("the `{id}` step should have run; ran {:?}", ids(ran)))
    }

    fn ids(ran: &[Step]) -> Vec<&str> {
        ran.iter().map(|step| step.id.as_str()).collect()
    }

    #[test]
    fn the_action_installs_the_archive_it_names_after_checking_its_digest() {
        let runner = Runner::new();
        let repo = fixture("T1", "rs", "silent");
        let ran = run_action(&runner, &repo, &runner.archive);

        let install = step(&ran, "install");
        assert_eq!(install.code, 0, "{}{}", install.stdout, install.stderr);
        let path_file = runner.read("github-path");
        let installed: Vec<&str> = path_file.lines().map(str::trim).collect();
        assert_eq!(installed.len(), 1, "the install puts one directory on PATH");
        let beside: Vec<String> = std::fs::read_dir(installed[0])
            .expect("the installed directory reads")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert_eq!(
            beside,
            ["weeder"],
            "the install extracts the one executable"
        );
        assert_eq!(
            ids(&ran),
            ["install", "review", "judge", "summary", "comment", "verdict"]
        );
        assert_eq!(step(&ran, "verdict").code, 0);
    }

    #[test]
    fn an_archive_whose_digest_does_not_match_is_never_opened() {
        let runner = Runner::new();
        let local = runner
            .archive
            .strip_prefix("file://")
            .expect("a file url")
            .to_string();
        let digest = format!("{local}.sha256");
        let written = std::fs::read_to_string(&digest).expect("the digest reads");
        let (hash, rest) = written.split_once(' ').expect("a digest and a name");
        let flipped: String = hash
            .chars()
            .map(|character| if character == '0' { '1' } else { '0' })
            .collect();
        std::fs::write(&digest, format!("{flipped} {rest}")).expect("the digest is replaced");

        let repo = fixture("T1", "rs", "silent");
        let ran = run_action(&runner, &repo, &runner.archive);
        let install = step(&ran, "install");
        assert_ne!(
            install.code, 0,
            "a digest that does not match stops the install"
        );
        assert!(
            install.stdout.contains("::error") && install.stdout.contains("SHA-256"),
            "and says why: {}{}",
            install.stdout,
            install.stderr
        );
        assert_eq!(
            ids(&ran),
            ["install"],
            "nothing runs after a refused install"
        );
        assert!(
            runner.read("github-path").trim().is_empty(),
            "nothing reaches PATH"
        );
    }

    #[test]
    fn a_blocked_change_fails_the_job_after_the_summary_and_the_comment_are_written() {
        let runner = Runner::new();
        let repo = fixture("T1", "rs", "fire");
        let ran = run_action(&runner, &repo, &runner.archive);

        assert_eq!(
            ids(&ran),
            ["install", "review", "judge", "summary", "comment", "verdict"]
        );
        assert_eq!(
            step(&ran, "judge").code,
            0,
            "judging records the verdict and goes on"
        );
        let summary = runner.read("summary.md");
        assert!(
            summary
                .lines()
                .next()
                .is_some_and(|line| line.contains("weeder"))
                && summary.contains("`T1`"),
            "the summary carries the comment's body:\n{summary}"
        );
        let comment = step(&ran, "comment");
        assert_eq!(
            comment.code, 0,
            "a comment that cannot be written never changes the job's result: {}{}",
            comment.stdout, comment.stderr
        );
        assert!(
            comment.stdout.contains("::warning"),
            "the job says the comment was not written: {}",
            comment.stdout
        );
        assert_eq!(
            step(&ran, "verdict").code,
            2,
            "weeder's exit code decides the job"
        );
    }

    #[test]
    fn a_run_weeder_could_not_finish_fails_the_job_with_three() {
        let runner = Runner::new();
        let repo = fixture("T1", "rs", "silent");
        repo.write("weeder.toml", "[this is not toml\n");
        repo.commit("a law nobody can read");
        let ran = run_action(&runner, &repo, &runner.archive);

        assert_eq!(
            ids(&ran),
            ["install", "review", "judge", "summary", "comment", "verdict"]
        );
        assert!(
            runner.read("summary.md").contains("could not judge"),
            "the summary says weeder judged nothing: {}",
            runner.read("summary.md")
        );
        assert_eq!(step(&ran, "verdict").code, 3);
    }

    /// An approving review starts the action again, and that run's context has
    /// no `github.base_ref`: GitHub fills it for `pull_request` alone. The
    /// pull request in the review's payload still names its base, and the
    /// checkout is the same merge ref a push gets, so the run judges the same
    /// change the same way.
    #[test]
    fn a_run_a_review_started_judges_the_change_a_push_would() {
        for (case, verdict) in [("fire", 2), ("silent", 0)] {
            let runner = Runner::new();
            let repo = fixture("T1", "rs", case);
            let ran = run_action_on(&runner, &repo, &runner.archive, &REVIEW);
            assert_eq!(
                ids(&ran),
                ["install", "review", "judge", "summary", "comment", "verdict"],
                "{}",
                ran.iter()
                    .map(|step| format!("{}: {}{}", step.id, step.stdout, step.stderr))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            assert_eq!(
                step(&ran, "judge").code,
                0,
                "{}",
                step(&ran, "judge").stdout
            );
            assert_eq!(step(&ran, "verdict").code, verdict, "the {case} fixture");
        }
    }

    /// With no token the reviews cannot be read, so nothing is accepted: the
    /// trailer's reason is shown and the block stands. The step says why and
    /// never fails the job itself.
    #[test]
    fn reviews_nobody_could_read_accept_nothing() {
        let runner = Runner::new();
        let repo = deleting_a_suite(ALLOWED);
        repo.git(&["update-ref", "refs/remotes/origin/main", "main"]);
        let ran = run_action(&runner, &repo, &runner.archive);
        let review = step(&ran, "review");
        assert_eq!(review.code, 0, "{}{}", review.stdout, review.stderr);
        assert!(
            review.stdout.contains("no review was read"),
            "the step says it read no review: {}",
            review.stdout
        );
        assert_eq!(step(&ran, "verdict").code, 2);
        assert!(
            runner
                .read("summary.md")
                .contains("the cases moved to the integration tier"),
            "the summary shows the reason waiting on an approval:\n{}",
            runner.read("summary.md")
        );
    }

    #[test]
    fn the_install_reads_the_release_this_action_names_out_of_its_own_manifest() {
        let action = action();
        let install = action["runs"]["steps"]
            .as_array()
            .and_then(|steps| steps.iter().find(|step| step["id"] == "install"))
            .expect("an install step");
        let script = install["run"].as_str().unwrap_or_default();
        assert!(
            script.contains("garden.json") && script.contains("install.binaries"),
            "with no archive named, the release is the one garden.json names at the action's ref:\n{script}"
        );
        let manifest: Value = serde_json::from_str(
            &std::fs::read_to_string(root().join("garden.json")).expect("garden.json reads"),
        )
        .expect("garden.json is json");
        for target in [
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-musl",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
        ] {
            assert!(
                manifest["install"]["binaries"][target].is_string(),
                "garden.json names a release for {target}"
            );
            assert!(
                script.contains(target),
                "the install picks {target} for the runner it is on"
            );
        }
    }
}

// --- reading markdown back ---------------------------------------------------

/// A comment as a CommonMark reader sees it: the html it hides, the prose
/// outside any list, and each list item.
#[derive(Debug, Default)]
struct Body {
    hidden: Vec<String>,
    prose: String,
    /// The prose outside any list, a paragraph at a time.
    paragraphs: Vec<String>,
    items: Vec<Item>,
}

#[derive(Debug, Default, Clone)]
struct Item {
    /// Everything the item shows, code included, in order.
    text: String,
    /// The text the item shows outside code.
    plain: Vec<String>,
    codes: Vec<String>,
    strong: Vec<String>,
    /// Each link's destination and the text it shows.
    links: Vec<(String, String)>,
    /// What the item shows, a line at a time: a hard break or the end of a
    /// paragraph ends one.
    lines: Vec<String>,
}

impl Item {
    /// The links into the judged commit's files, the places a finding names.
    fn places(&self) -> Vec<(String, String)> {
        self.links
            .iter()
            .filter(|link| link.0.starts_with(LINKS))
            .cloned()
            .collect()
    }
}

impl Body {
    fn read(markdown: &str) -> Body {
        let mut body = Body::default();
        let mut item: Option<Item> = None;
        let mut strong = false;
        let mut link: Option<(String, String)> = None;
        // Whether the last line or paragraph has ended, so the next text starts
        // a new one.
        let mut ended = true;
        for event in Parser::new(markdown) {
            match event {
                Event::Html(html) | Event::InlineHtml(html) => body.hidden.push(html.to_string()),
                Event::Start(Tag::Item) => {
                    item = Some(Item::default());
                    ended = true;
                }
                Event::End(TagEnd::Item) => body.items.extend(item.take()),
                Event::Start(Tag::Strong) => strong = true,
                Event::End(TagEnd::Strong) => strong = false,
                Event::Start(Tag::Link { dest_url, .. }) => {
                    link = Some((dest_url.to_string(), String::new()))
                }
                Event::End(TagEnd::Link) => {
                    if let (Some(item), Some(link)) = (item.as_mut(), link.take()) {
                        item.links.push(link);
                    }
                }
                Event::Text(ref text) | Event::Code(ref text) => {
                    let code = matches!(event, Event::Code(_));
                    let text = text.to_string();
                    if let Some(link) = link.as_mut() {
                        link.1.push_str(&text);
                    }
                    match item.as_mut() {
                        Some(item) => {
                            item.text.push_str(&text);
                            match item.lines.last_mut() {
                                Some(line) if !ended => line.push_str(&text),
                                _ => item.lines.push(text.clone()),
                            }
                            ended = false;
                            if code {
                                item.codes.push(text);
                            } else {
                                if strong {
                                    item.strong.push(text.clone());
                                }
                                item.plain.push(text);
                            }
                        }
                        None => {
                            body.prose.push_str(&text);
                            match body.paragraphs.last_mut() {
                                Some(paragraph) if !ended => paragraph.push_str(&text),
                                _ => body.paragraphs.push(text.clone()),
                            }
                            ended = false;
                        }
                    }
                }
                Event::SoftBreak => match item.as_mut() {
                    Some(item) => {
                        item.text.push(' ');
                        if let Some(line) = item.lines.last_mut() {
                            line.push(' ');
                        }
                    }
                    None => {
                        body.prose.push(' ');
                        if let Some(paragraph) = body.paragraphs.last_mut() {
                            paragraph.push(' ');
                        }
                    }
                },
                Event::HardBreak => {
                    match item.as_mut() {
                        Some(item) => item.text.push(' '),
                        None => body.prose.push(' '),
                    }
                    ended = true;
                }
                Event::End(TagEnd::Paragraph) => {
                    match item.as_mut() {
                        Some(item) => item.text.push(' '),
                        None => body.prose.push(' '),
                    }
                    ended = true;
                }
                _ => {}
            }
        }
        body
    }
}

/// A message as a reader sees it, with the case of its letters set aside: the
/// comment opens each line of a finding with a capital, and the log does not.
fn folded(text: &str) -> String {
    unticked(text).to_lowercase()
}

/// What a reader sees of a message, which is the message without the backticks
/// that mark its code, and with its spaces as a renderer folds them.
fn unticked(text: &str) -> String {
    text.replace('`', "")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// The results in the order the comment lists them: blocks, then warnings,
/// then notes, and inside a level in the order the log keeps.
fn ordered(log: &Value) -> Vec<Value> {
    let mut results = log["runs"][0]["results"]
        .as_array()
        .expect("a run carries results")
        .clone();
    let rank = |result: &Value| match result["level"].as_str() {
        Some("error") => 0,
        Some("warning") => 1,
        _ => 2,
    };
    results.sort_by_key(rank);
    results
}

fn place(result: &Value) -> (String, Option<u64>, Option<u64>) {
    let location = &result["locations"][0]["physicalLocation"];
    (
        location["artifactLocation"]["uri"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        location["region"]["startLine"].as_u64(),
        location["region"]["endLine"].as_u64(),
    )
}

/// `<links>/<path>#L<line>`, the path written byte by byte with everything but
/// the unreserved characters and `/` percent-encoded, and a range as
/// `#L<start>-L<end>`.
fn expected_link(path: &str, line: Option<u64>, end: Option<u64>) -> String {
    let mut encoded = String::new();
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    match (line, end) {
        (Some(line), Some(end)) => format!("{LINKS}/{encoded}#L{line}-L{end}"),
        (Some(line), None) => format!("{LINKS}/{encoded}#L{line}"),
        _ => format!("{LINKS}/{encoded}"),
    }
}
