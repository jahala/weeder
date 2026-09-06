#!/usr/bin/env bash
# Evidence for calibration c6: the report's first sentence says the verdict is
# pending until the independent re-grade agrees at or above ninety percent on
# every sample, the generator decides that by reading the audit file, and
# calibration-bar.sh refuses the unqualified sentence while no re-grade stands
# behind it.
#
# The point of writing the sentence from a file is that nobody can promote a
# provisional verdict by editing the report, so that is what is tested: a report
# is edited to drop the qualification and the bar script has to refuse it; an
# audit at the bar is written and the same script has to insist the
# qualification comes off. Both directions, or the wording is decoration.
#
# The generator's half is proved on a history built for the purpose, where the
# audit is the only thing that changes between two runs. The suite in
# xtask/tests/verdict.rs runs last for the cases around the edges: a sample too
# small to rest on, one sample of two under the bar, a file that draws no sample,
# and a run that does not clear the bar at all.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

report="docs/calibration-2026-09.md"
# Every audit record the repository carries, packet, response and transcript
# files aside, colon-separated: the wording is written from all of them.
audit="$(grep -l '^Blind: yes' docs/calibration-audit*.md 2>/dev/null | grep -v '\.packet\.\|\.response\.\|transcript' | tr '\n' ':' | sed 's/:$//')"
corpus="docs/calibration/corpus.toml"
provisional="weeder ships as a gate, pending the independent re-grade:"
confirmed="weeder ships as a gate:"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the report cannot be read. The check refuses to pass on an unread file." >&2
  exit 3
}
command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }
[ -f "$report" ] || { echo "$report is missing: there is no verdict to read" >&2; exit 1; }

scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT
status=0

# The generator, on a history built for the purpose: one blocking commit, one
# classification, and the audit as the only thing that changes between runs.
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

judge() {
  cargo run -q -p xtask -- calibrate \
    --corpus "$bench/corpus.toml" \
    --out "$1" \
    --judgements "$bench/judgements.toml" \
    --first-run "$bench/first-run.toml" \
    --audit "$bench/audit.md" >/dev/null
}
first_sentence() {
  python3 -c 'import sys
lines = open(sys.argv[1], encoding="utf-8").read().splitlines()[1:]
print(next((line for line in lines if line.strip()), ""))' "$1"
}

judge "$bench/pending.md"
sentence="$(first_sentence "$bench/pending.md")"
case "$sentence" in
  "$provisional"*) ;;
  *)
    echo "with no re-grade written the generator should call the verdict pending, and it said: $sentence" >&2
    status=1
    ;;
esac

printf '# the re-grade\n\nBlind: yes\n\n| Sample | Re-graded | Agreed |\n|---|---|---|\n| blocked commits | 20 | 17 |\n' \
  > "$bench/audit.md"
judge "$bench/under.md"
sentence="$(first_sentence "$bench/under.md")"
case "$sentence" in
  "$provisional"*) ;;
  *)
    echo "a re-grade agreeing on 85 percent is under the bar and the verdict should still be pending: $sentence" >&2
    status=1
    ;;
esac

printf '# the re-grade\n\nBlind: yes\n\n| Sample | Re-graded | Agreed |\n|---|---|---|\n| blocked commits | 20 | 19 |\n| recall cases | 20 | 20 |\n' \
  > "$bench/audit.md"
judge "$bench/confirmed.md"
sentence="$(first_sentence "$bench/confirmed.md")"
case "$sentence" in
  "$confirmed"*) ;;
  *)
    echo "every sample agrees at or above ninety percent and the qualification should be gone: $sentence" >&2
    status=1
    ;;
esac
if [ "$(first_sentence "$bench/pending.md")" = "$(first_sentence "$bench/confirmed.md")" ]; then
  echo "the sentence did not change when the re-grade did, so it is not written from that file" >&2
  status=1
fi

# The bar script, against a report edited to say what the re-grade does not.
promoted="$scratch/promoted.md"
python3 - "$report" "$promoted" "$provisional" "$confirmed" <<'PY'
import sys

report_path, out_path, provisional, confirmed = sys.argv[1:5]
text = open(report_path, encoding="utf-8").read()
if provisional in text:
    text = text.replace(provisional, confirmed, 1)
open(out_path, "w", encoding="utf-8").write(text)
PY

bar() {
  WEEDER_CALIBRATION_REPORT="$1" \
  WEEDER_CALIBRATION_AUDIT="$2" \
  WEEDER_CALIBRATION_CORPUS="$corpus" \
  WEEDER_CALIBRATION_METRIC="$scratch/metric.json" \
    bash scripts/check/calibration-bar.sh
}

if bar "$promoted" "$scratch/no-such-audit.md" > "$scratch/promoted.out" 2> "$scratch/promoted.err"; then
  echo "calibration-bar.sh accepted '$confirmed' with no re-grade behind it, so the wording is unguarded" >&2
  status=1
elif ! grep -q "with no re-grade behind it" "$scratch/promoted.err"; then
  echo "calibration-bar.sh refused the promoted verdict for some other reason:" >&2
  cat "$scratch/promoted.err" >&2
  status=1
fi

printf '# the re-grade\n\nBlind: yes\n\n| Sample | Re-graded | Agreed |\n|---|---|---|\n| blocked commits | 20 | 17 |\n' \
  > "$scratch/under-audit.md"
if bar "$promoted" "$scratch/under-audit.md" > /dev/null 2> "$scratch/under.err"; then
  echo "calibration-bar.sh accepted '$confirmed' behind a re-grade that agrees on 85 percent" >&2
  status=1
elif ! grep -q "with no re-grade behind it" "$scratch/under.err"; then
  echo "calibration-bar.sh refused the under-bar re-grade for some other reason:" >&2
  cat "$scratch/under.err" >&2
  status=1
fi

printf '# the re-grade\n\nBlind: yes\n\n| Sample | Re-graded | Agreed |\n|---|---|---|\n| blocked commits | 20 | 19 |\n' \
  > "$scratch/good-audit.md"
if ! bar "$promoted" "$scratch/good-audit.md" > /dev/null 2> "$scratch/good.err"; then
  echo "calibration-bar.sh refused '$confirmed' with a re-grade at the bar behind it:" >&2
  cat "$scratch/good.err" >&2
  status=1
fi
# The stale case is a pending sentence over a re-grade at the bar. The report in
# the tree may already read confirmed, so the pending sentence is written into a
# probe rather than assumed of the file.
pending="$scratch/pending.md"
sed "1,3s/^weeder ships as a gate: /weeder ships as a gate, pending the independent re-grade: /" "$report" > "$pending"
if bar "$pending" "$scratch/good-audit.md" > /dev/null 2> "$scratch/stale.err"; then
  echo "calibration-bar.sh accepted a pending sentence while the re-grade agrees at the bar, so the wording can go stale unnoticed" >&2
  status=1
elif ! grep -q "agrees at the bar and the first sentence still calls the verdict pending" "$scratch/stale.err"; then
  echo "calibration-bar.sh refused the stale wording for some other reason:" >&2
  cat "$scratch/stale.err" >&2
  status=1
fi

# And the shipped file, against the re-grade the repository actually has.
python3 - "$report" "$audit" "$provisional" "$confirmed" <<'PY' || status=1
import os
import re
import sys

report_path, audit_path, provisional, confirmed = sys.argv[1:5]
report = open(report_path, encoding="utf-8").read()
complaints = []

sentence = next((line for line in report.splitlines()[1:] if line.strip()), "")
samples = []
audit_paths = [one for one in audit_path.split(":") if one and os.path.exists(one)]
present = bool(audit_paths)
for one_path in audit_paths:
    for line in open(one_path, encoding="utf-8").read().splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) < 3:
            continue
        try:
            samples.append((cells[0].strip("*").strip(), int(cells[1]), int(cells[2])))
        except ValueError:
            continue

stands = bool(samples) and all(
    regraded >= 20 and agreed <= regraded and agreed * 100.0 / regraded >= 90.0
    for _, regraded, agreed in samples
)
if sentence.startswith("weeder does not ship as a gate:"):
    print("the calibration does not clear the bar, so the re-grade decides nothing about its wording")
    raise SystemExit(0)
if stands and not sentence.startswith(confirmed):
    complaints.append(
        f"{audit_path} agrees at the bar on every sample and the first sentence still reads: {sentence!r}"
    )
if not stands and not sentence.startswith(provisional):
    complaints.append(
        f"no re-grade stands behind the classification and the first sentence reads: {sentence!r}"
    )
if not stands:
    reason = report.split("\n\n")[2] if len(report.split("\n\n")) > 2 else ""
    if not any(one_path in reason for one_path in audit_paths) and audit_path not in reason:
        complaints.append(
            "the sentence after the verdict does not name the file the qualification is waiting on"
        )
if re.search(r"^weeder ships as a gate, pending", report, re.M) and stands:
    complaints.append("the wording is behind the audit file it is written from")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(
    f"the shipped verdict reads {'as confirmed' if stands else 'as pending'}, "
    f"which is what {audit_path} earns it"
)
PY

# The cases around the edges.
cargo test --package xtask --test verdict || status=1

[ "$status" -eq 0 ] || exit "$status"
echo "the verdict's wording is written from the re-grade, and the bar script refuses an unqualified verdict that no re-grade stands behind"
