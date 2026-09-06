//! A planted case is a real instance of its rule's shape, or it is no case.
//!
//! Recall only means something where every case is one weed could have caught.
//! A site the injector read a shape into that was never there is worthless
//! twice over: a hit on it inflates the number, and a miss on it is charged to
//! the binary for an anti-pattern it was never shown, which is worse, because a
//! broken injector then reads as a gap in the product.
//!
//! Both shapes the blind re-grade of the sample named are built here as
//! histories the campaign walks. A python project whose only version string is
//! the release it calls itself: moving it pins nothing and is no dependency
//! change. A suite whose case is named after the failure it is about: putting
//! `Exception` in its brackets breaks a signature rather than loosening a
//! claim. Neither may become a case, and each has a history beside it where the
//! shape is real, so the refusal is the scanner reading rather than the rule
//! being switched off.
//!
//! The third history is what the scanner can only refuse after the write: a Go
//! suite written a case to a line, where the marker that turns a case off lands
//! past the end of the case it was meant for. That case is unplantable, it is
//! counted as such per rule and language beside the misses, and it is named in
//! neither column.

mod common;

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use common::{Bench, Repo, Run};

/// The library the histories are written around, so a commit has code to change
/// and the suites have something to be about.
const LOADER: &str = "\
def load(text):
    \"\"\"Read a source into the value the report is built from.\"\"\"
    stripped = text.strip()
    if not stripped:
        return None
    return stripped


def widest(text):
    lines = text.split(\"\\n\")
    return max(len(line) for line in lines)
";

const REPORT: &str = "\
def summarise(rows):
    return \", \".join(str(row) for row in rows)


def total(rows):
    counted = 0
    for row in rows:
        counted += row
    return counted
";

/// A suite whose case is named after the failure it is about, and which makes
/// no claim about a failure anywhere. The only brackets carrying the word are
/// the ones the case declares itself with.
const NAMED_AFTER_THE_FAILURE: &str = "\
from probe.loader import load


class TestLoader:
    def test_empty_source_raises(self):
        assert load(\"\") is None

    def test_a_source_loads(self):
        assert load(\"one\") == \"one\"

    def test_a_source_is_stripped(self):
        assert load(\" one \") == \"one\"
";

/// The same suite, claiming a failure by name inside a case, which is the shape
/// the rule is about.
const CLAIMS_THE_FAILURE: &str = "\
import pytest

from probe.loader import load


class TestLoader:
    def test_a_source_loads(self):
        assert load(\"one\") == \"one\"

    def test_an_empty_source_is_refused(self):
        with pytest.raises(ValueError, match=\"source\"):
            load(None)

    def test_a_source_is_stripped(self):
        assert load(\" one \") == \"one\"
";

/// A manifest that states the release the project calls itself, and pins
/// nothing.
const OWN_VERSION_ONLY: &str = "\
from setuptools import setup

setup(
    name=\"probe\",
    version=\"1.2.3\",
    packages=[\"probe\"],
)
";

/// A manifest that pins somebody else's code, under the release the project
/// calls itself, so the pin is the second version in the file and not the
/// first.
const PINS_A_DEPENDENCY: &str = "\
[project]
name = \"probe\"
version = \"1.2.3\"
dependencies = [
    \"httpx==0.27.0\",
    \"click==8.1.7\",
]
";

fn python_history(manifest: (&str, &str), suite: &str) -> Repo {
    let repo = Repo::init();
    repo.write(manifest.0, manifest.1);
    repo.write("probe/loader.py", LOADER);
    repo.write("probe/report.py", REPORT);
    repo.write("tests/test_loader.py", suite);
    repo.commit("the project begins");
    for step in 1..=8 {
        repo.write(
            "probe/report.py",
            &format!(
                "{REPORT}\n\ndef widest_{step}(rows):\n    return max(rows, default={step})\n"
            ),
        );
        repo.commit(&format!("the project grows, step {step}"));
    }
    repo
}

/// A Go suite that writes each of its cases to a line. There is nowhere inside
/// such a case to put the marker that turns it off: the line after it belongs
/// to the file rather than to the case.
const CASES_ON_ONE_LINE: &str = "\
package probe

import \"testing\"

func TestOne(t *testing.T) { if One() != 1 { t.Fatal(\"one\") } }

func TestTwo(t *testing.T) { if Two() != 2 { t.Fatal(\"two\") } }

func TestThree(t *testing.T) { if Three() != 3 { t.Fatal(\"three\") } }
";

const COUNTS: &str = "\
package probe

func One() int {
	return 1
}

func Two() int {
	return 2
}

func Three() int {
	return 3
}
";

fn go_history() -> Repo {
    let repo = Repo::init();
    repo.write("go.mod", "module probe\n\ngo 1.21\n");
    repo.write("probe.go", COUNTS);
    repo.write("probe_test.go", CASES_ON_ONE_LINE);
    repo.write(
        "shape/shape.go",
        "package shape\n\nfunc Wide(text string) int {\n\treturn len(text)\n}\n",
    );
    repo.commit("the project begins");
    for step in 1..=8 {
        repo.write(
            "shape/shape.go",
            &format!(
                "package shape\n\nfunc Wide(text string) int {{\n\treturn len(text) + {step}\n}}\n"
            ),
        );
        repo.commit(&format!("the project grows, step {step}"));
    }
    repo
}

/// The binary the campaign judges with, built once for the whole suite so the
/// runs do not queue behind one another for the build lock.
fn weed() -> &'static PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the workspace root")
            .to_path_buf();
        let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
            .current_dir(&root)
            .args(["build", "--release", "--quiet", "--bin", "weed"])
            .status()
            .expect("cargo should be on PATH");
        assert!(status.success(), "the release binary should build");
        let binary = common::binary()
            .parent()
            .and_then(|debug| debug.parent())
            .expect("the target directory")
            .join("release/weed");
        assert!(binary.exists(), "{} should be built", binary.display());
        binary
    })
}

/// Walk one history for one rule in one language, and hand back the run with
/// the numbers it wrote.
fn campaign(bench: &Bench, name: &str, repo: &Repo, rule: &str, language: &str) -> Run {
    let binary = weed().display().to_string();
    bench.mutate(
        name,
        repo,
        &[
            "--rule",
            rule,
            "--language",
            language,
            "--cases",
            "4",
            "--commits",
            "9",
            "--binary",
            &binary,
        ],
    )
}

/// How many cases a run planted for a rule in a language.
fn planted(bench: &Bench, rule: &str, language: &str) -> usize {
    bench
        .planted()
        .iter()
        .filter(|case| case.rule == rule && case.language == language)
        .count()
}

#[test]
fn a_version_a_project_states_for_itself_is_no_dependency_change() {
    let bench = Bench::new();
    let repo = python_history(("setup.py", OWN_VERSION_ONLY), CLAIMS_THE_FAILURE);
    campaign(&bench, "probe", &repo, "D1", "py").succeeded();

    assert_eq!(
        planted(&bench, "D1", "py"),
        0,
        "the only version in the tree is the release the project calls itself, which pins \
         nothing:\n{}",
        bench.report()
    );
    assert!(
        !bench.report().contains("- D1 · py ·"),
        "a case that was never a dependency change is named as a miss:\n{}",
        bench.report()
    );
}

#[test]
fn a_manifest_that_pins_a_dependency_is_still_a_case() {
    let bench = Bench::new();
    let repo = python_history(("pyproject.toml", PINS_A_DEPENDENCY), CLAIMS_THE_FAILURE);
    campaign(&bench, "probe", &repo, "D1", "py").succeeded();

    assert!(
        planted(&bench, "D1", "py") > 0,
        "a manifest that pins somebody else's code is a site, and the run found none:\n{}",
        bench.report()
    );
    assert_eq!(
        bench.unplantable("D1", "py"),
        0,
        "a real pin was written and the scanner would not read it back:\n{}",
        bench.report()
    );
}

#[test]
fn a_case_named_after_a_failure_is_no_claim_about_one() {
    let bench = Bench::new();
    let repo = python_history(
        ("pyproject.toml", PINS_A_DEPENDENCY),
        NAMED_AFTER_THE_FAILURE,
    );
    campaign(&bench, "probe", &repo, "T6", "py").succeeded();

    assert_eq!(
        planted(&bench, "T6", "py"),
        0,
        "the word is in the case's name and nowhere in its body, so there is no claim to \
         weaken:\n{}",
        bench.report()
    );
    assert!(
        !bench.report().contains("- T6 · py ·"),
        "a broken signature is named as a claim weed missed:\n{}",
        bench.report()
    );
}

#[test]
fn a_claim_that_names_the_failure_is_still_a_case() {
    let bench = Bench::new();
    let repo = python_history(("pyproject.toml", PINS_A_DEPENDENCY), CLAIMS_THE_FAILURE);
    campaign(&bench, "probe", &repo, "T6", "py").succeeded();

    assert!(
        planted(&bench, "T6", "py") > 0,
        "a case that claims a failure by name is a site, and the run found none:\n{}",
        bench.report()
    );
}

#[test]
fn a_marker_that_lands_past_its_case_is_unplantable_and_is_no_miss() {
    let bench = Bench::new();
    let repo = go_history();
    campaign(&bench, "probe", &repo, "T3", "go").succeeded();

    assert_eq!(
        planted(&bench, "T3", "go"),
        0,
        "the marker landed outside every case, and the case was counted anyway:\n{}",
        bench.report()
    );
    assert!(
        bench.unplantable("T3", "go") > 0,
        "nothing was counted as unplantable, so the scanner never read the tree back:\n{}",
        bench.report()
    );
    assert!(
        !bench.report().contains("- T3 · go ·"),
        "a case that was never planted is named as a miss:\n{}",
        bench.report()
    );
}

#[test]
fn the_report_carries_the_unplantable_count_beside_the_misses() {
    let bench = Bench::new();
    let repo = go_history();
    campaign(&bench, "probe", &repo, "T3", "go").succeeded();
    let report = bench.report();

    let row = report
        .lines()
        .find(|line| line.starts_with("| T3 | block | go |"))
        .unwrap_or_else(|| panic!("the report should carry a row for T3 in go:\n{report}"));
    let columns: Vec<&str> = row.split('|').map(str::trim).collect();
    assert!(
        report
            .contains("| Rule | Level | Language | Cases | Hits | Misses | Unplantable | Recall |"),
        "the table has no unplantable column beside its misses:\n{report}"
    );
    assert_eq!(
        columns.get(7).copied(),
        Some(bench.unplantable("T3", "go").to_string().as_str()),
        "the row does not carry what the run counted: {row}"
    );
}
