#!/usr/bin/env bash
# Evidence for calibration c1: `cargo xtask calibrate` judges the last 200
# commits of each corpus repository against their parents, writes
# docs/calibration-2026-09.md in the shape the loop describes with the verdict
# first, and leaves the repositories it read exactly as it found them.
#
# The window is not taken on trust. This script asks git itself, in each source
# repository, how many commits of the default branch have a parent and are not
# merges, and refuses a report whose count is anything else. The repositories are
# fingerprinted before the run and again after it, refs, HEAD and working tree,
# so "untouched" is measured rather than asserted. And the report the run writes
# is compared byte for byte with the one in the tree: a calibration that no
# longer describes the history it names is not a calibration.
#
# The suites in xtask/tests run last. They judge a history built for the purpose,
# where every commit is known, which is the half this corpus cannot prove: what
# an unclassified block counts as, what a window of two holds, and that two runs
# over one history write the same bytes.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

report="docs/calibration-2026-09.md"
corpus="docs/calibration/corpus.toml"
window=200

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the report cannot be read. The check refuses to pass on an unread file." >&2
  exit 3
}
command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }
[ -f "$corpus" ] || { echo "$corpus is missing: calibration names no repositories to judge" >&2; exit 1; }

scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT
status=0

# The corpus, as name and path, read out of the file the run reads.
python3 - "$corpus" > "$scratch/corpus" <<'PY'
import re, sys

text = open(sys.argv[1], encoding="utf-8").read()
for block in text.split("[[repo]]")[1:]:
    name = re.search(r'^\s*name\s*=\s*"([^"]+)"', block, re.M)
    path = re.search(r'^\s*path\s*=\s*"([^"]+)"', block, re.M)
    if name and path:
        print(f"{name.group(1)}\t{path.group(1)}")
PY
[ -s "$scratch/corpus" ] || { echo "$corpus names no repositories" >&2; exit 1; }

# The default branch each repository's upstream names, the window it holds, and
# a fingerprint of everything a run could disturb.
: > "$scratch/expected"
while IFS=$'\t' read -r name path; do
  [ -d "$path" ] || { echo "the corpus names $name at $path, and there is no checkout there" >&2; exit 3; }
  reference="$(git -C "$path" symbolic-ref --quiet refs/remotes/origin/HEAD || true)"
  if [ -z "$reference" ]; then
    for candidate in refs/heads/main refs/heads/master; do
      if git -C "$path" rev-parse --verify --quiet "$candidate" >/dev/null; then reference="$candidate"; break; fi
    done
  fi
  [ -n "$reference" ] || reference="$(git -C "$path" rev-parse --symbolic-full-name HEAD)"

  judged=0
  for sha in $(git -C "$path" rev-list --no-merges -n "$window" "$reference"); do
    if git -C "$path" rev-parse --verify --quiet "$sha^" >/dev/null; then
      judged=$((judged + 1))
    fi
  done
  printf '%s\t%s\t%s\n' "$name" "$reference" "$judged" >> "$scratch/expected"
  {
    git -C "$path" rev-parse --symbolic-full-name HEAD
    git -C "$path" rev-parse HEAD
    git -C "$path" for-each-ref --format='%(objectname) %(refname)'
    git -C "$path" worktree list --porcelain
    git -C "$path" status --porcelain=v1 --untracked-files=all
  } > "$scratch/before-$name"
done < "$scratch/corpus"

# The run, writing where nothing else can have written.
fresh="$scratch/calibration.md"
if ! cargo xtask calibrate --out "$fresh"; then
  echo "cargo xtask calibrate failed" >&2
  exit 1
fi

while IFS=$'\t' read -r name path; do
  {
    git -C "$path" rev-parse --symbolic-full-name HEAD
    git -C "$path" rev-parse HEAD
    git -C "$path" for-each-ref --format='%(objectname) %(refname)'
    git -C "$path" worktree list --porcelain
    git -C "$path" status --porcelain=v1 --untracked-files=all
  } > "$scratch/after-$name"
  if ! diff -q "$scratch/before-$name" "$scratch/after-$name" >/dev/null; then
    echo "$name at $path changed while calibration read it: a ref, HEAD, a worktree or the working tree moved. Calibration writes nothing there, so either it does now or somebody else was working in that checkout." >&2
    diff "$scratch/before-$name" "$scratch/after-$name" >&2 || true
    status=1
  fi
done < "$scratch/corpus"

[ -f "$report" ] || { echo "$report is missing: the calibration file is the loop's product" >&2; exit 1; }
if ! diff -q "$fresh" "$report" >/dev/null; then
  echo "$report is not what 'cargo xtask calibrate' writes today. The corpus has moved on; run the calibration again and classify whatever is newly blocked." >&2
  diff "$report" "$fresh" | head -40 >&2 || true
  status=1
fi

# The shape, read out of the file rather than assumed.
python3 - "$report" "$scratch/expected" <<'PY' || status=1
import re, sys

report = open(sys.argv[1], encoding="utf-8").read()
lines = report.splitlines()
expected = [line.split("\t") for line in open(sys.argv[2], encoding="utf-8").read().splitlines()]
complaints = []

if not lines or not lines[0].startswith("# calibration"):
    complaints.append("the file does not open with a calibration title")

# The verdict is the first sentence: the first line of prose under the title,
# and it says ship or it says kill.
body = [line for line in lines[1:] if line.strip()]
verdict = body[0] if body else ""
if not (verdict.startswith("weed ships as a gate:") or verdict.startswith("weed does not ship as a gate:")):
    complaints.append(f"the first sentence is not the verdict: {verdict!r}")

# One section per repository, with the count git itself gives.
for name, reference, judged in expected:
    heading = re.search(rf"^## {re.escape(name)}, (\d+) commits judged, (\d+) blocked, (\d+) warned$", report, re.M)
    if heading is None:
        complaints.append(f"{name} has no section of its own in the report")
        continue
    if heading.group(1) != judged:
        complaints.append(
            f"{name}: the report judged {heading.group(1)} commits and git puts the window at {judged}"
        )
    if reference not in report:
        complaints.append(f"{name}: the report does not name the ref it judged, {reference}")

# Every blocked commit is a row with a rule, a class and a reason.
rows = re.findall(r"^\| `([0-9a-f]{7,})` (.*?) \| ([^|]+) \| ([^|]+) \| ([^|]+) \|$", report, re.M)
blocked = sum(int(found.group(1)) for found in re.finditer(r"^## \S+, \d+ commits judged, (\d+) blocked", report, re.M))
if len(rows) != blocked:
    complaints.append(f"the repository tables carry {len(rows)} blocked commits and the headings count {blocked}")
for sha, subject, ruleids, classification, why in rows:
    if not re.fullmatch(r"(?:[A-Z]\d+)(?:, [A-Z]\d+)*", ruleids.strip()):
        complaints.append(f"{sha}: '{ruleids.strip()}' is not a list of rule ids")
    if classification.strip() not in {"true positive", "acceptable", "false positive", "unclassified"}:
        complaints.append(f"{sha}: '{classification.strip()}' is not a classification")
    if len(why.strip()) < 20:
        complaints.append(f"{sha}: the reasoning is too short to be a reason: {why.strip()!r}")

for heading in ("## How this was measured", "## The rules that ran", "## Totals",
                "## Precision and the allowance rate", "## The rules that blocked"):
    if heading not in report:
        complaints.append(f"the report has no '{heading}' section")

if "| **pooled** |" not in report:
    complaints.append("the totals carry no pooled row")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"{len(rows)} blocked commits in {len(expected)} repositories, every one of them classified in a table")
PY

# The half a live corpus cannot prove: a history built for the purpose.
cargo test --package xtask --test calibrate || status=1

[ "$status" -eq 0 ] || exit "$status"
echo "cargo xtask calibrate: the window is git's own, the report is the shape the loop describes with the verdict first, and every repository it read is as it was found"
