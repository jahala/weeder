#!/usr/bin/env bash
# Evidence for calibration c9: the paragraph after the verdict prints the
# verdict's floor beside the ledger's own share.
#
# The share the verdict rests on is a share of judgements, and the judgements are
# the builder's. A reader handed that one number has to take the reading behind
# it on trust. So the paragraph after it prints the same measurement again with
# every `false-positive` a blind auditor recorded believed over the ledger: the
# harshest reading this repository has written down, beside the reading it
# shipped. Two numbers a reader can hold apart are worth more than one they have
# to accept.
#
# Nothing here is taken from the generator. The two shares are recomputed from
# the report's own repository tables and the blind re-grades' own blocked
# samples, and the paragraph has to print exactly those. Then a history built for
# the purpose is judged several times with only the re-grade's table moving under
# it, because a number that does not move when its source does was typed rather
# than read. The suite in xtask/tests/floor.rs runs last, for the cases this
# corpus does not happen to contain.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

report="docs/calibration-2026-09.md"
# Every audit record the repository carries, packet, response and transcript
# files aside: the floor is drawn from whichever of them declare themselves blind.
audit="$(ls docs/calibration-audit*.md 2>/dev/null | grep -v '\.packet\.\|\.response\.\|transcript' | tr '\n' ':' | sed 's/:$//')"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the report cannot be added up. The check refuses to pass on unchecked arithmetic." >&2
  exit 3
}
command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }
[ -f "$report" ] || { echo "$report is missing: there is no paragraph to read a floor out of" >&2; exit 1; }

status=0

# The shipped report, added up. Which two numbers the paragraph has to print is
# worked out here from the report's tables and the re-grades' tables, so the
# check reads the same evidence the generator did and never asks it what it wrote.
python3 - "$report" "$audit" <<'PY' || status=1
import os
import re
import sys

report_path, audit_path = sys.argv[1:3]
report = open(report_path, encoding="utf-8").read()
complaints = []

AGAINST = {"false positive", "unclassified"}


def cells(line):
    line = line.strip()
    if not line.startswith("|"):
        return None
    found = [cell.strip() for cell in line.strip("|").split("|")]
    if all(re.fullmatch(r":?-{3,}:?", cell.replace(" ", "")) for cell in found):
        return None
    return found


def plain(cell):
    return cell.strip().strip("`").strip("*").strip()


def share(part, whole):
    return 0.0 if whole == 0 else round(part * 100.0 / whole, 2)


# Every blocked commit the report accounts for, and how many commits it judged.
judged = 0
blocks = []
repo = None
recall = False
for line in report.splitlines():
    if line.strip() == "<!-- recall:begin -->":
        recall = True
    if recall:
        continue
    heading = re.match(r"^## ([A-Za-z0-9_-]+), (\d+) commits judged,", line)
    if heading:
        repo = heading.group(1)
        judged += int(heading.group(2))
        continue
    row = cells(line)
    if not repo or not row or len(row) < 4:
        continue
    named = re.match(r"`([0-9a-f]{7,40})`", row[0])
    if not named:
        continue
    classification = plain(row[2])
    if classification not in AGAINST and classification not in {"true positive", "acceptable"}:
        continue
    blocks.append((repo, named.group(1), classification))

if not blocks:
    complaints.append(f"{report_path} names no blocked commit, so there is no floor to draw")
if judged == 0:
    complaints.append(f"{report_path} judged nothing, so neither share exists")

# Every `false-positive` a re-grade that declares itself blind recorded.
def declaration(text):
    for line in text.splitlines():
        stripped = "".join(mark for mark in line if mark not in "*_#>`").strip().lower()
        if not stripped.startswith("blind:"):
            continue
        answer = stripped[len("blind:"):].split(";")[0].split(",")[0].split(".")[0].strip()
        if answer in ("yes", "true"):
            return "blind"
        if answer in ("no", "false"):
            return "sighted"
    return None


def regraded_blocks(text):
    rows = []
    inside = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("## "):
            inside = "blocked commit" in stripped.lower()
            continue
        row = cells(line)
        if not inside or not row or len(row) < 3:
            continue
        name, sha, verdict = plain(row[0]), plain(row[1]), plain(row[2]).lower().replace(" ", "-")
        if not name or not sha or not verdict:
            continue
        if not all(mark in "0123456789abcdef" for mark in sha):
            continue
        rows.append((name, sha, verdict))
    return rows


blind_files = []
refused = set()
for path in [one for one in audit_path.split(":") if one and os.path.exists(one)]:
    text = open(path, encoding="utf-8").read()
    rows = regraded_blocks(text)
    if declaration(text) != "blind" or not rows:
        continue
    blind_files.append(path)
    for name, sha, verdict in rows:
        if verdict == "false-positive":
            refused.add((name, sha))

ledger_count = sum(1 for _, _, classification in blocks if classification in AGAINST)
floor_count = 0
for name, sha, classification in blocks:
    against = classification in AGAINST
    seen = any(
        name == other and (sha.startswith(short) or short.startswith(sha))
        for other, short in refused
    )
    if against or seen:
        floor_count += 1

paragraphs = [one for one in report.split("\n\n") if one.strip()]
after = paragraphs[2].strip() if len(paragraphs) > 2 else ""
if not after:
    complaints.append(f"{report_path} carries no paragraph after the verdict")

if blind_files:
    wanted = [
        f"The floor under that share is {share(floor_count, judged):.2f} percent, {floor_count} of the {judged} commits judged",
        f"the ledger's own reading is {share(ledger_count, judged):.2f} percent, {ledger_count} of the same {judged}",
    ] + blind_files
else:
    wanted = [
        "No blind re-grade has read a blocked commit back",
        f"the ledger's own {share(ledger_count, judged):.2f} percent, {ledger_count} of {judged} commits judged",
    ]

for phrase in wanted:
    if phrase not in after:
        complaints.append(f"the paragraph after the verdict does not say {phrase!r}")

# The floor is a floor: it can only be at or above the share the verdict rests on.
if floor_count < ledger_count:
    complaints.append(
        f"the floor counts {floor_count} blocks and the ledger counts {ledger_count}; a floor cannot be under the share it floors"
    )

# And it belongs before any section of the file, in the paragraph after the
# verdict rather than somewhere further down.
for phrase in ("The floor under that share is", "No blind re-grade has read a blocked commit back"):
    at = report.find(phrase)
    if at != -1 and "\n\n## " in report[:at]:
        complaints.append(f"{phrase!r} first appears after a section heading, not after the verdict")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(
    f"the shipped report prints a floor of {share(floor_count, judged):.2f} percent beside a ledger reading of "
    f"{share(ledger_count, judged):.2f} percent, both recomputed here from {len(blocks)} blocked commits and "
    f"{len(refused)} blind refusals"
)
PY

# A history built for the purpose, judged with only the re-grade's table moving
# under it. A floor that does not move when the table does was typed.
scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT

source="$scratch/source"
mkdir -p "$source/tests"
git -C "$source" init --quiet --initial-branch=main
git -C "$source" config user.name "weed measurements"
git -C "$source" config user.email "measurements@weed.invalid"
commit() {
  git -C "$source" add -A
  GIT_AUTHOR_DATE="2026-09-06T09:00:00+00:00" GIT_COMMITTER_DATE="2026-09-06T09:00:00+00:00" \
    git -C "$source" commit --quiet -m "$1"
}
suite() {
  local cases="$1" out=""
  local case=0
  while [ "$case" -lt "$cases" ]; do
    out="$out#[test]\nfn case_$case() {\n    assert_eq!($case + 1, $((case + 1)));\n}\n\n"
    case=$((case + 1))
  done
  printf "$out"
}
for file in alpha beta gamma; do suite 3 > "$source/tests/$file.rs"; done
commit "the repository begins"
shas=""
for file in alpha beta gamma; do
  suite 2 > "$source/tests/$file.rs"
  commit "one case fewer in $file"
  shas="$shas $(git -C "$source" rev-parse main)"
done
set -- $shas

bench="$scratch/bench"
mkdir -p "$bench"
: > "$bench/first-run.toml"
printf '[[repo]]\nname = "probe"\nsource = "%s"\ntip = "%s"\n' "$source" "$3" > "$bench/corpus.toml"
{
  printf '[[commit]]\nrepo = "probe"\nsha = "%s"\nclassification = "false-positive"\nreasoning = "the finding says the case is unwatched and another file watches it"\n\n' "$1"
  printf '[[commit]]\nrepo = "probe"\nsha = "%s"\nclassification = "acceptable"\nreasoning = "the case really went and the change was right to take it"\n\n' "$2"
  printf '[[commit]]\nrepo = "probe"\nsha = "%s"\nclassification = "true-positive"\nreasoning = "the case really went and nothing else holds what it held"\n' "$3"
} > "$bench/judgements.toml"

# Write one blind re-grade whose blocked sample refuses the commits named.
regrade() {
  local path="$1"
  shift
  {
    printf '# the re-grade\n\nBlind: yes\n\n'
    printf '| Sample | Re-graded | Agreed |\n|---|---|---|\n| blocked commits | 20 | 20 |\n\n'
    printf '## Blocked Commit Sample\n\n| Repo | Commit | Auditor verdict | Reasoning |\n|---|---|---|---|\n'
    local one
    for one in "$@"; do
      printf '| probe | `%s` | false-positive | the auditor read the finding and the change |\n' "${one:0:10}"
    done
  } > "$path"
}
judge() {
  local out="$1"
  shift
  local arguments=(calibrate --corpus "$bench/corpus.toml" --out "$out"
    --judgements "$bench/judgements.toml" --first-run "$bench/first-run.toml")
  local one
  for one in "$@"; do
    arguments+=(--audit "$one")
  done
  cargo run -q -p xtask -- "${arguments[@]}" >/dev/null
}
after() {
  python3 -c 'import sys
paragraphs = [one for one in open(sys.argv[1], encoding="utf-8").read().split("\n\n") if one.strip()]
print(paragraphs[2].strip() if len(paragraphs) > 2 else "")' "$1"
}
carries() {
  local what="$1" phrase="$2" paragraph="$3"
  case "$paragraph" in
    *"$phrase"*) ;;
    *)
      echo "$what: the paragraph after the verdict does not say '$phrase':" >&2
      echo "  $paragraph" >&2
      status=1
      ;;
  esac
}

# One refusal the ledger already counts moves nothing; one it does not lifts the
# floor by exactly that block; refusing everything puts the floor at all of them.
regrade "$bench/audit-blind.md" "$1"
judge "$bench/same.md" "$bench/audit-blind.md"
same="$(after "$bench/same.md")"
carries "a refusal the ledger already counts" "The floor under that share is 33.33 percent, 1 of the 3 commits judged" "$same"
carries "a refusal the ledger already counts" "the ledger's own reading is 33.33 percent, 1 of the same 3" "$same"

regrade "$bench/audit-blind.md" "$2"
judge "$bench/one-more.md" "$bench/audit-blind.md"
one_more="$(after "$bench/one-more.md")"
carries "a refusal the ledger does not count" "The floor under that share is 66.67 percent, 2 of the 3 commits judged" "$one_more"

regrade "$bench/audit-blind.md" "$1" "$2" "$3"
judge "$bench/all.md" "$bench/audit-blind.md"
all="$(after "$bench/all.md")"
carries "an auditor who refuses every block" "The floor under that share is 100.00 percent, 3 of the 3 commits judged" "$all"

if [ "$same" = "$one_more" ] || [ "$one_more" = "$all" ]; then
  echo "the paragraph did not change when the re-grade's table did, so the floor is not read from it" >&2
  status=1
fi

# No blind re-grade at all, and a sighted one that refuses everything: neither
# draws a floor, because a sighted auditor hands the builder's answer back.
judge "$bench/none.md" "$bench/no-such-audit.md"
none="$(after "$bench/none.md")"
carries "no re-grade at all" "No blind re-grade has read a blocked commit back" "$none"
carries "no re-grade at all" "the ledger's own 33.33 percent, 1 of 3 commits judged" "$none"

regrade "$bench/audit-sighted.md" "$1" "$2" "$3"
python3 - "$bench/audit-sighted.md" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read().replace("Blind: yes", "Blind: no")
open(path, "w", encoding="utf-8").write(text)
PY
judge "$bench/sighted.md" "$bench/audit-sighted.md"
sighted="$(after "$bench/sighted.md")"
carries "a sighted re-grade that refuses every block" "No blind re-grade has read a blocked commit back" "$sighted"

# The cases around the edges.
cargo test --package xtask --test floor || status=1

[ "$status" -eq 0 ] || exit "$status"
echo "the paragraph after the verdict prints the verdict's floor beside the ledger's own share, recomputed here from the report's tables and drawn by the generator from the blind re-grade's"
