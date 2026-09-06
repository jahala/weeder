#!/usr/bin/env bash
# Evidence for calibration c8: the report's first paragraph says, right after the
# verdict, whether the re-grade the number rests on was taken with the builder's
# classification in sight or without it, and it says it from the `Blind:` line
# each audit file writes about itself.
#
# The caveat exists because an agreement is worth what the auditor could not see.
# A second party who read the builder's class beside each case before judging
# agreed with something already in front of them, so forty of forty taken that
# way reads stronger than it is. A report that prints the agreement and not the
# condition it was taken under prints a stronger number than it has.
#
# So the sentence has to be a reading of the files rather than a description of
# them, and that is what is tested here. A history is built for the purpose and
# the same measurement is run over it several times, with the audit files as the
# only thing that changes: one sighted re-grade, a blind one at the bar beside
# it, a blind one under the bar beside it, and a file whose name says blind and
# whose declaration says otherwise. Then the shipped report, against the
# declarations the repository's own audit files carry, re-derived here rather
# than taken from the generator. The suite in xtask/tests/sight.rs runs last.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

report="docs/calibration-2026-09.md"
# Every audit record the repository carries, packet, response and transcript
# files aside: the caveat is written from all of them.
audit="$(ls docs/calibration-audit*.md 2>/dev/null | grep -v '\.packet\.\|\.response\.\|transcript' | tr '\n' ':' | sed 's/:$//')"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the report cannot be read. The check refuses to pass on an unread file." >&2
  exit 3
}
command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }
[ -f "$report" ] || { echo "$report is missing: there is no paragraph to read" >&2; exit 1; }

scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT
status=0

SIGHTED="The re-grade behind it is a sighted one"
BLIND="The re-grade behind it is blind"

# A history with one blocking commit in it, so the run has something to classify
# and the verdict is a ship rather than a kill.
source="$scratch/source"
mkdir -p "$source/tests"
git -C "$source" init --quiet --initial-branch=main
git -C "$source" config user.name "weeder measurements"
git -C "$source" config user.email "measurements@weeder.invalid"
commit() {
  git -C "$source" add -A
  GIT_AUTHOR_DATE="2026-09-06T09:00:00+00:00" GIT_COMMITTER_DATE="2026-09-06T09:00:00+00:00" \
    git -C "$source" commit --quiet -m "$1"
}
printf '#[test]\nfn one() {\n    assert_eq!(1, 1);\n}\n\n#[test]\nfn two() {\n    assert_eq!(2, 2);\n}\n' \
  > "$source/tests/unit.rs"
commit "the repository begins"
printf '#[test]\nfn one() {\n    assert_eq!(1, 1);\n}\n' > "$source/tests/unit.rs"
commit "one case fewer"
blocked="$(git -C "$source" rev-parse main)"

bench="$scratch/bench"
mkdir -p "$bench"
: > "$bench/first-run.toml"
printf '[[repo]]\nname = "probe"\nsource = "%s"\ntip = "%s"\n' "$source" "$blocked" > "$bench/corpus.toml"
cat > "$bench/judgements.toml" <<LEDGER
[[commit]]
repo = "probe"
sha = "$blocked"
classification = "true-positive"
reasoning = "a case really did go, and that is what T1 watches"
LEDGER

# Write one re-grade: its declaration, or `none` for a file that makes none, and
# a sample of twenty with however many of them agreed.
regrade() {
  local path="$1" declaration="$2" agreed="$3"
  {
    printf '# the re-grade\n\n'
    [ "$declaration" = "none" ] || printf 'Blind: %s\n\n' "$declaration"
    printf '| Sample | Re-graded | Agreed |\n|---|---|---|\n| blocked commits | 20 | %s |\n' "$agreed"
  } > "$path"
}

# Judge the probe with the audit files named, and hand back the paragraph the
# report opens with.
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
opening() {
  python3 -c 'import sys
paragraphs = [one for one in open(sys.argv[1], encoding="utf-8").read().split("\n\n") if one.strip()]
print(paragraphs[1].strip() if len(paragraphs) > 1 else "")' "$1"
}
# The paragraph has to carry the phrase; names the case in the complaint.
carries() {
  local what="$1" phrase="$2" paragraph="$3"
  case "$paragraph" in
    *"$phrase"*) ;;
    *)
      echo "$what: the opening paragraph does not say '$phrase':" >&2
      echo "  $paragraph" >&2
      status=1
      ;;
  esac
}
lacks() {
  local what="$1" phrase="$2" paragraph="$3"
  case "$paragraph" in
    *"$phrase"*)
      echo "$what: the opening paragraph should not say '$phrase':" >&2
      echo "  $paragraph" >&2
      status=1
      ;;
  esac
}

# One re-grade, declaring itself sighted.
regrade "$bench/audit-a.md" "no; the auditor could read the builder's classification" 20
judge "$bench/sighted.md" "$bench/audit-a.md"
sighted_only="$(opening "$bench/sighted.md")"
carries "a sighted re-grade alone" "$SIGHTED" "$sighted_only"
carries "a sighted re-grade alone" "could read the builder's class beside each case before judging" "$sighted_only"
carries "a sighted re-grade alone" "audit-a.md" "$sighted_only"
lacks "a sighted re-grade alone" "$BLIND" "$sighted_only"

# The caveat is in the paragraph the verdict opens, and nothing stands between
# them.
python3 - "$bench/sighted.md" "$SIGHTED" <<'PY' || status=1
import sys

path, caveat = sys.argv[1:3]
text = open(path, encoding="utf-8").read()
paragraphs = [one for one in text.split("\n\n") if one.strip()]
opening = paragraphs[1].strip() if len(paragraphs) > 1 else ""
if caveat not in opening:
    print("the caveat is not in the paragraph the verdict opens", file=sys.stderr)
    raise SystemExit(1)
at = opening.index(caveat)
before = opening[:at]
if not before.startswith("weeder ships as a gate") and not before.startswith(
    "weeder does not ship as a gate"
):
    print(f"the paragraph does not open with the verdict: {before!r}", file=sys.stderr)
    raise SystemExit(1)
# One sentence and one space: the caveat follows the verdict and nothing follows
# the caveat into a paragraph of its own.
if not before.endswith(". "):
    print(f"the caveat does not follow the verdict sentence: {before!r}", file=sys.stderr)
    raise SystemExit(1)
if "\n" in opening:
    print(f"the verdict and the caveat are not one paragraph: {opening!r}", file=sys.stderr)
    raise SystemExit(1)
PY

# A blind re-grade at the bar beside it: the caveat says so and names both.
regrade "$bench/audit-b.md" "yes" 20
judge "$bench/blind.md" "$bench/audit-a.md" "$bench/audit-b.md"
blind_at_bar="$(opening "$bench/blind.md")"
carries "a blind re-grade at the bar" "$BLIND" "$blind_at_bar"
carries "a blind re-grade at the bar" "audit-b.md" "$blind_at_bar"
carries "a blind re-grade at the bar" "audit-a.md" "$blind_at_bar"
lacks "a blind re-grade at the bar" "$SIGHTED" "$blind_at_bar"

# The same blind re-grade, under the bar: it carries nothing, and the agreement
# behind the number is a sighted one still.
regrade "$bench/audit-b.md" "yes" 9
judge "$bench/short.md" "$bench/audit-a.md" "$bench/audit-b.md"
blind_short="$(opening "$bench/short.md")"
carries "a blind re-grade under the bar" "$SIGHTED" "$blind_short"
carries "a blind re-grade under the bar" "audit-b.md is under the bar" "$blind_short"
lacks "a blind re-grade under the bar" "$BLIND" "$blind_short"

# The declaration decides it, and not the file's name.
regrade "$bench/audit-blind.md" "no" 20
judge "$bench/named.md" "$bench/audit-blind.md"
named="$(opening "$bench/named.md")"
carries "a file named blind that declares otherwise" "$SIGHTED" "$named"
lacks "a file named blind that declares otherwise" "$BLIND" "$named"
regrade "$bench/audit-blind.md" "yes" 20
judge "$bench/redeclared.md" "$bench/audit-blind.md"
redeclared="$(opening "$bench/redeclared.md")"
carries "the same file, redeclared blind" "$BLIND" "$redeclared"
if [ "$named" = "$redeclared" ]; then
  echo "the paragraph did not change when the declaration did, so it is not read from that line" >&2
  status=1
fi

# A file that declares nothing is not read as either.
regrade "$bench/audit-quiet.md" "none" 20
judge "$bench/quiet.md" "$bench/audit-quiet.md"
quiet="$(opening "$bench/quiet.md")"
carries "a re-grade that declares nothing" 'declares no `Blind:` line either way' "$quiet"
lacks "a re-grade that declares nothing" "$SIGHTED" "$quiet"
lacks "a re-grade that declares nothing" "$BLIND" "$quiet"

# And no re-grade at all.
judge "$bench/none.md" "$bench/no-such-audit.md"
none="$(opening "$bench/none.md")"
carries "no re-grade at all" "No re-grade is recorded" "$none"

# The shipped report, against the declarations the repository's own audit files
# carry. Which sentence it has to hold is worked out here from those files, so
# the check reads the same evidence the generator did and never asks it what it
# wrote.
python3 - "$report" "$audit" <<'PY' || status=1
import os
import sys

report_path, audit_path = sys.argv[1:3]
report = open(report_path, encoding="utf-8").read()
complaints = []

SIGHTED = "The re-grade behind it is a sighted one"
BLIND = "The re-grade behind it is blind"
SAMPLE_FLOOR = 20
AGREEMENT_BAR = 90.0


def declaration(text):
    """The file's own `Blind:` line: 'blind', 'sighted', or nothing."""
    for line in text.splitlines():
        plain = "".join(mark for mark in line if mark not in "*_#>`").strip().lower()
        if not plain.startswith("blind:"):
            continue
        answer = plain[len("blind:") :].split(";")[0].split(",")[0].split(".")[0].strip()
        if answer in ("yes", "true"):
            return "blind"
        if answer in ("no", "false"):
            return "sighted"
    return None


def samples(text):
    found = []
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) < 3:
            continue
        try:
            found.append((int(cells[1]), int(cells[2])))
        except ValueError:
            continue
    return found


def stands(found):
    return bool(found) and all(
        regraded >= SAMPLE_FLOOR
        and agreed <= regraded
        and agreed * 100.0 / regraded >= AGREEMENT_BAR
        for regraded, agreed in found
    )


records = []
for path in [one for one in audit_path.split(":") if one and os.path.exists(one)]:
    text = open(path, encoding="utf-8").read()
    records.append((path, declaration(text), stands(samples(text))))

paragraphs = [one for one in report.split("\n\n") if one.strip()]
opening = paragraphs[1].strip() if len(paragraphs) > 1 else ""

wanted = []
unwanted = []
if not records:
    wanted.append("No re-grade is recorded")
else:
    blind_at_bar = [path for path, sight, stood in records if sight == "blind" and stood]
    sighted = [path for path, sight, _ in records if sight == "sighted"]
    if blind_at_bar:
        wanted.append(BLIND)
        wanted.extend(blind_at_bar)
        wanted.extend(sighted)
        unwanted.append(SIGHTED)
    else:
        if sighted:
            wanted.append(SIGHTED)
            wanted.append("could read the builder's class beside each case before judging")
            wanted.extend(sighted)
        for path, sight, stood in records:
            if sight == "blind" and not stood:
                wanted.append(f"{path} is under the bar")
            if sight is None:
                wanted.append(f"{path} declares no `Blind:` line either way")
        unwanted.append(BLIND)

for phrase in wanted:
    if phrase not in opening:
        complaints.append(f"the opening paragraph does not say {phrase!r}")
for phrase in unwanted:
    if phrase in opening:
        complaints.append(f"the opening paragraph should not say {phrase!r}")

if not opening.startswith("weeder ships as a gate") and not opening.startswith(
    "weeder does not ship as a gate"
):
    complaints.append(f"the report does not open with a verdict: {opening!r}")
for phrase in (SIGHTED, BLIND, "No re-grade is recorded"):
    at = report.find(phrase)
    if at != -1 and "\n\n## " in report[:at]:
        complaints.append(f"{phrase!r} first appears after a section heading, not in the opening")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(
    "the shipped report's opening paragraph says what "
    + " and ".join(path for path, _, _ in records)
    + " declare about themselves"
)
PY

# The cases around the edges.
cargo test --package xtask --test sight || status=1

[ "$status" -eq 0 ] || exit "$status"
echo "the paragraph the verdict opens says whether the re-grade behind it was sighted or blind, and it says it from the audit files' own declarations"
