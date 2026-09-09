#!/usr/bin/env bash
# Evidence for calibration c3: the measurement garden.json publishes is this
# one, and its latest answer is in the tree beside the report it was read from.
#
# The command is not read and approved, it is run: whatever string the manifest
# carries is executed from the repository root, and it has to be the bar script.
# Then the answer it wrote is compared with the answer already committed. A
# metric whose published number is older than the file it summarises is a number
# nobody can act on, so a difference fails rather than quietly updating.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

manifest="garden.json"
expected="bash scripts/check/calibration-bar.sh"
result="docs/calibration/metric.json"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the manifest cannot be read. The check refuses to pass on an unread manifest." >&2
  exit 3
}
command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }
[ -f "$manifest" ] || { echo "$manifest is missing: weeder publishes no manifest for the umbrella to read" >&2; exit 1; }

# The umbrella's manifest contract carries the metric as the command itself, so
# what the number means is read off the answer the command writes, below, rather
# than off a second field beside it.
command="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("metric") or "")' "$manifest")"

status=0
if [ "$command" != "$expected" ]; then
  echo "garden.json names '$command' as its metric command, and calibration's measurement is '$expected'" >&2
  status=1
fi
[ "$status" -eq 0 ] || exit "$status"

[ -f "$result" ] || {
  echo "$result is missing: the metric's latest answer is not in the tree" >&2
  exit 1
}
if git check-ignore --quiet "$result"; then
  echo "$result is ignored by git, so the latest result would never reach a commit" >&2
  exit 1
fi

# The answer already in the tree, kept aside while the metric is run again.
committed="$(mktemp)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$committed"' EXIT
cp "$result" "$committed"

if ! bash -c "$command" > /dev/null; then
  cp "$committed" "$result"
  echo "the metric command garden.json names did not pass: weeder publishes a measurement it does not meet" >&2
  exit 1
fi

if ! diff -q "$committed" "$result" > /dev/null; then
  diff "$committed" "$result" >&2 || true
  cp "$committed" "$result"
  echo "$result is not what the metric command answers today. Run '$command' and commit what it writes." >&2
  exit 1
fi

python3 - "$result" <<'PY' || exit 1
import json
import sys

result = json.load(open(sys.argv[1], encoding="utf-8"))
complaints = []
for field in ("metric", "commits_judged", "false_positives", "share_percent", "bar_percent", "repos"):
    if field not in result:
        complaints.append(f"the committed result carries no {field}")
if not complaints:
    if result["share_percent"] >= result["bar_percent"]:
        complaints.append(
            f"the committed result is {result['share_percent']}% against a {result['bar_percent']}% bar"
        )
    if not result["repos"]:
        complaints.append("the committed result names no repositories, so no repository's own share is published")
for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(
    f"{result['metric']}: {result['share_percent']}% of {result['commits_judged']} commits, "
    f"over {len(result['repos'])} repositories"
)
PY

echo "garden.json's metric runs $expected, and $result is the answer it gives today"
