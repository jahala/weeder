#!/usr/bin/env bash
# Evidence for recall.tend2.html c4: every planted case is a genuine instance of
# its rule's shape at the planted site. After the shape is written into the tree
# the injector's own scanner reads the tree back, and a case whose shape is not
# there is unplantable: counted per rule and language beside the misses, and
# charged to neither column.
#
# Three things are proved here, in the order they matter. A history is built
# carrying both shapes the blind re-grade of the sample named — a python project
# whose only version string is the release it calls itself, and a suite whose
# case is named after the failure it is about — beside a history where both
# shapes are real, so a refusal is the scanner reading rather than the rule
# being switched off. Then the count: a Go suite that writes each case to a line
# has nowhere inside a case to put the marker that turns one off, and the case
# that lands past it has to be counted as unplantable and named in no miss list.
# Then the shipped corpus, small: the column is in every row, its numbers are
# the run's, and every miss the run kept is read back off disk by this script,
# in its own words, and held to the two shapes the re-grade named.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the runs cannot be read back. The check refuses to pass on an unread report." >&2
  exit 3
}
command -v cargo >/dev/null 2>&1 || { echo "cargo is not on PATH, so the campaign cannot be run." >&2; exit 3; }
command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }

scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT
status=0

# ------------------------------------------------------------ the scanners --
# The injector's own suites first: what it reads a site out of, and what it
# reads back afterwards, are the whole measurement's independence from weeder.
cargo test --release --quiet -p xtask --bins
cargo test --release --quiet -p xtask --test shape

# -------------------------------------------------------------- the bench --
# Two repositories: one writing both shapes as they really are, one writing the
# two the re-grade named and a Go suite the marker cannot be planted inside of.
build() { # <root> <kind>
  local repo="$1" kind="$2"
  mkdir -p "$repo/probe" "$repo/tests" "$repo/shape"
  git -C "$repo" init --quiet --initial-branch=main
  git -C "$repo" config user.name "weeder measurements"
  git -C "$repo" config user.email "measurements@weeder.invalid"

  cat > "$repo/probe/loader.py" <<'PY'
def load(text):
    """Read a source into the value the report is built from."""
    stripped = text.strip()
    if not stripped:
        return None
    return stripped


def widest(text):
    return max(len(line) for line in text.split("\n"))
PY
  cat > "$repo/probe/report.py" <<'PY'
def summarise(rows):
    return ", ".join(str(row) for row in rows)


def total(rows):
    counted = 0
    for row in rows:
        counted += row
    return counted
PY
  cat > "$repo/probe.go" <<'GO'
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
GO
  cat > "$repo/shape/shape.go" <<'GO'
package shape

func Wide(text string) int {
	return len(text)
}
GO
  printf 'module probe\n\ngo 1.21\n' > "$repo/go.mod"

  if [ "$kind" = "real" ]; then
    cat > "$repo/pyproject.toml" <<'TOML'
[project]
name = "probe"
version = "1.2.3"
dependencies = [
    "httpx==0.27.0",
    "click==8.1.7",
]
TOML
    cat > "$repo/tests/test_loader.py" <<'PY'
import pytest

from probe.loader import load


class TestLoader:
    def test_a_source_loads(self):
        assert load("one") == "one"

    def test_an_empty_source_is_refused(self):
        with pytest.raises(ValueError, match="source"):
            load(None)

    def test_a_source_is_stripped(self):
        assert load(" one ") == "one"
PY
    cat > "$repo/probe_test.go" <<'GO'
package probe

import "testing"

func TestOne(t *testing.T) {
	if One() != 1 {
		t.Fatal("one")
	}
}

func TestTwo(t *testing.T) {
	if Two() != 2 {
		t.Fatal("two")
	}
}
GO
  else
    # The only version string in the tree is the release the project calls
    # itself, which pins nothing.
    cat > "$repo/setup.py" <<'PY'
from setuptools import setup

setup(
    name="probe",
    version="1.2.3",
    packages=["probe"],
)
PY
    # The word a claim about a failure is written with is in a case's name and
    # nowhere in a case's body.
    cat > "$repo/tests/test_loader.py" <<'PY'
from probe.loader import load


class TestLoader:
    def test_empty_source_raises(self):
        assert load("") is None

    def test_a_source_loads(self):
        assert load("one") == "one"

    def test_a_source_is_stripped(self):
        assert load(" one ") == "one"
PY
    # Every case is written to a line, so there is no body to turn one off in.
    cat > "$repo/probe_test.go" <<'GO'
package probe

import "testing"

func TestOne(t *testing.T) { if One() != 1 { t.Fatal("one") } }

func TestTwo(t *testing.T) { if Two() != 2 { t.Fatal("two") } }

func TestThree(t *testing.T) { if Three() != 3 { t.Fatal("three") } }
GO
  fi

  commit "$repo" "the project begins"
  for step in 1 2 3 4 5 6 7 8; do
    printf 'package shape\n\nfunc Wide(text string) int {\n\treturn len(text) + %s\n}\n' "$step" > "$repo/shape/shape.go"
    commit "$repo" "the project grows, step $step"
  done
}

commit() { # <root> <message>
  git -C "$1" add -A
  GIT_AUTHOR_DATE="2026-09-06T09:00:00+00:00" GIT_COMMITTER_DATE="2026-09-06T09:00:00+00:00" \
    git -C "$1" commit --quiet -m "$2"
}

build "$scratch/real" real
build "$scratch/flat" flat
{
  printf '[[repo]]\nname = "real"\nsource = "%s"\ntip = "%s"\n\n' \
    "$scratch/real" "$(git -C "$scratch/real" rev-parse main)"
  printf '[[repo]]\nname = "flat"\nsource = "%s"\ntip = "%s"\n' \
    "$scratch/flat" "$(git -C "$scratch/flat" rev-parse main)"
} > "$scratch/corpus.toml"

if ! WEEDER_RECALL_CACHE="$scratch/cache" cargo run -q -p xtask -- mutate \
  --corpus "$scratch/corpus.toml" \
  --rule D1 --rule T6 --rule T3 \
  --language py --language go \
  --cases 8 \
  --commits 9 \
  --cases-dir "$scratch/cache/cases" \
  --out "$scratch/bench.md" \
  --json "$scratch/bench.json" > "$scratch/bench.log" 2>&1; then
  echo "the campaign refused to run over the history built for it:" >&2
  cat "$scratch/bench.log" >&2
  exit 3
fi

python3 - "$scratch/bench.md" "$scratch/bench.json" <<'PY' || status=1
import json
import sys

report = open(sys.argv[1], encoding="utf-8").read()
run = json.load(open(sys.argv[2], encoding="utf-8"))
cases = run["cases"]
complaints = []

unplantable = {
    (held["rule"], held["language"]): held["cases"] for held in run["unplantable"]
}


def planted(rule, language, repository):
    return [
        case
        for case in cases
        if case["rule"] == rule
        and case["language"] == language
        and case["repository"] == repository
    ]


# The shapes the re-grade named, and the Go suite no marker fits inside: none of
# the three may become a case, and none may be named as a miss.
REFUSED = [
    ("D1", "py", "a version string that pins nothing"),
    ("T6", "py", "a case named after the failure it is about"),
    ("T3", "go", "a marker with no case body to land in"),
]
for rule, language, shape in REFUSED:
    counted = planted(rule, language, "flat")
    if counted:
        complaints.append(
            f"{rule} in {language}: {len(counted)} cases were planted on {shape}, "
            f"and every one of them is a case about nothing"
        )
    for line in report.splitlines():
        if line.startswith(f"- {rule} · {language} · flat "):
            complaints.append(f"{rule} in {language}: {shape} is named as a miss: {line}")

# And the same three rules, on a history where the shapes are real. A scanner
# that refused everything would pass the half above and nothing here.
for rule, language, shape in REFUSED:
    if not planted(rule, language, "real"):
        complaints.append(
            f"{rule} in {language}: no case was planted where the shape is real, so the "
            f"refusal of {shape} says nothing"
        )

# The marker that landed past its case is counted, and counted once for each
# time it was written.
if unplantable.get(("T3", "go"), 0) < 1:
    complaints.append(
        "no T3 case in go was counted unplantable, so the tree was never read back after "
        "the marker was written into it"
    )

# The column, beside the misses, carrying what the run counted.
header = "| Rule | Level | Language | Cases | Hits | Misses | Unplantable | Recall |"
if header not in report:
    complaints.append("the recall table carries no unplantable column beside its misses")
for line in report.splitlines():
    columns = [column.strip() for column in line.split("|")]
    if len(columns) < 3 or not columns[1][:1].isalpha() or not columns[1][1:].isdigit():
        continue
    if len(columns) != 10:
        complaints.append(f"a row of the table is not eight columns wide: {line}")
        continue
    rule, language, written = columns[1], columns[3], columns[7]
    counted = unplantable.get((rule, language), 0)
    if written != str(counted):
        complaints.append(
            f"{rule} in {language}: the table says {written} unplantable and the run counted "
            f"{counted}"
        )

for complaint in complaints:
    print(complaint, file=sys.stderr)
raise SystemExit(1 if complaints else 0)
PY

# ------------------------------------------------------- the shipped corpus --
# The same read, on the corpus the measurement is taken over, and every miss it
# keeps held to the two shapes the re-grade named — read off disk here, in this
# script's own words, rather than taken from the campaign.
corpus="docs/calibration/corpus.toml"
go="docs/calibration/corpus-go.toml"
[ -f "$corpus" ] || { echo "$corpus is missing: the campaign names no repositories to walk" >&2; exit 1; }
[ -f "$go" ] || { echo "$go is missing: the Go column has no history to be measured on" >&2; exit 1; }

cases="${WEEDER_SHAPE_CASES:-6}"
commits="${WEEDER_SHAPE_COMMITS:-60}"
if ! cargo run -q -p xtask -- mutate \
  --cases "$cases" \
  --commits "$commits" \
  --cases-dir "$scratch/kept" \
  --out "$scratch/shipped.md" \
  --json "$scratch/shipped.json" > "$scratch/shipped.log" 2>&1; then
  echo "the campaign could not run over the shipped corpus. every repository has to hand over the commit it is pinned at." >&2
  cat "$scratch/shipped.log" >&2
  exit 3
fi

python3 - "$scratch/shipped.md" "$scratch/shipped.json" "$scratch/kept" <<'PY' || status=1
import json
import pathlib
import re
import sys

report = open(sys.argv[1], encoding="utf-8").read()
run = json.load(open(sys.argv[2], encoding="utf-8"))
kept = pathlib.Path(sys.argv[3])
cases = run["cases"]
complaints = []

unplantable = {
    (held["rule"], held["language"]): held["cases"] for held in run["unplantable"]
}

FLOOR = 100
if len(cases) < FLOOR:
    complaints.append(
        f"the run over the shipped corpus planted {len(cases)} cases, too few to say what it "
        f"reads back"
    )

rows = 0
for line in report.splitlines():
    columns = [column.strip() for column in line.split("|")]
    if len(columns) < 3 or not columns[1][:1].isalpha() or not columns[1][1:].isdigit():
        continue
    rows += 1
    if len(columns) != 10:
        complaints.append(f"a row of the table is not eight columns wide: {line}")
        continue
    rule, language, written = columns[1], columns[3], columns[7]
    if written != str(unplantable.get((rule, language), 0)):
        complaints.append(
            f"{rule} in {language}: the table says {written} unplantable and the run counted "
            f"{unplantable.get((rule, language), 0)}"
        )
if rows < 40:
    complaints.append(f"the table carries {rows} rows, and every rule is measured in four languages")


def moved(case):
    """The lines one kept case wrote and took away, read off the pair on disk."""
    before, after = case / "before", case / "after"
    written, taken = [], []
    for path in sorted(set(after.rglob("*")) | set(before.rglob("*"))):
        inside = path.relative_to(after if path.is_relative_to(after) else before)

        def text(side):
            try:
                return (side / inside).read_text(encoding="utf-8", errors="replace").splitlines()
            except OSError:
                return []

        now, held = text(after), text(before)
        written.extend(line for line in now if line not in held)
        taken.extend(line for line in held if line not in now)
    return written, taken


VERSION = re.compile(r"(?<![\w.])\d+\.\d+\.\d+(?![\w.])")
WORD = re.compile(r"[A-Za-z_][\w.-]*")


def only_the_projects_own_version(line):
    """Whether the one version on a line is the release the file states for
    itself, which pins nothing and is no dependency change."""
    found = VERSION.search(line)
    if found is None:
        return False
    words = WORD.findall(line[: found.start()])
    return bool(words) and all(word.strip("_").lower() == "version" for word in words)


DECLARES = ("def ", "async def ", "func ", "fn ", "function ")

# Every miss the run kept, read back here. A miss on a case that was never the
# rule's shape is the one thing the count may not carry, and these are the two
# shapes the re-grade of the sample found it carrying.
read = 0
for note in sorted(kept.rglob("case.json")):
    case = json.load(open(note, encoding="utf-8"))
    written, taken = moved(note.parent)
    named = f"{case['rule']} · {case['repository']} {case['commit'][:9]}"
    read += 1
    if not written and not taken:
        complaints.append(f"{named}: the case moved no line")
        continue
    if case["rule"] == "D1":
        for line in written:
            if only_the_projects_own_version(line):
                complaints.append(
                    f"{named}: the case moved the release the project calls itself, which pins "
                    f"nothing: {line.strip()}"
                )
    if case["rule"] == "T6":
        for line in written:
            if line.lstrip().startswith(DECLARES):
                complaints.append(
                    f"{named}: the case rewrote a declaration rather than a claim: {line.strip()}"
                )

for complaint in complaints[:20]:
    print(complaint, file=sys.stderr)
print(
    f"{len(cases)} cases planted over the shipped corpus, "
    f"{sum(unplantable.values())} shapes written and read back without the shape in them, "
    f"{read} kept misses read off disk",
    file=sys.stderr,
)
raise SystemExit(1 if complaints else 0)
PY

[ "$status" -eq 0 ] || exit "$status"
echo "every case the campaign counts is a shape its own scanner found in the tree after it was written: the two shapes the blind re-grade named are planted by neither history that carries them, both are planted where they are real, a marker that landed past its case is counted unplantable and named in no miss list, and the count sits in every row of the table beside the misses"
