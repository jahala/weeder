#!/usr/bin/env bash
# Evidence for calibration c4: `cargo xtask suppressions` counts `Weed-allow:`
# trailers per hundred commits on each garden repository from the day guard is
# installed, counts none before that day, and the calibration file reports that
# rate beside precision with the true-positive count on the same line.
#
# Every number the measurement gives is recomputed here from git directly, in a
# scratch fetched at the same pins the corpus names rather than in any checkout
# on this machine: the commit that first brought a guard bundle into the tree,
# the commits from there to the pin, the trailers on them, and the trailers
# written before. The marker the install is found by is read out of weed's own
# source, so the script and the binary cannot drift apart into agreeing about the
# wrong string.
#
# No garden repository has installed guard yet, so every rate today is zero, and
# a measurement that can only answer zero proves nothing. The suite in
# xtask/tests/suppressions.rs runs last against a history built for the purpose:
# allowances before the hooks arrive, the install, allowances after.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

report="docs/calibration-2026-09.md"
corpus="docs/calibration/corpus.toml"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the rates cannot be recomputed. The check refuses to pass on unchecked arithmetic." >&2
  exit 3
}
command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }

# The line `weed guard install` writes into every bundle, taken from the source
# that writes it rather than copied.
marker="$(sed -n 's/^pub const BINARY_MARKER: &str = "\(.*\)";$/\1/p' src/core/guard.rs)"
[ -n "$marker" ] || {
  echo "src/core/guard.rs no longer declares BINARY_MARKER, so there is no way to find the day guard was installed" >&2
  exit 3
}

scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT
status=0

# The corpus, fetched at its pins into a scratch this script owns.
bash "$root/scripts/corpus-scratch.sh" "$scratch/corpus-scratch" > "$scratch/corpus"
[ -s "$scratch/corpus" ] || { echo "$corpus names no repositories" >&2; exit 1; }

# git's own answer, repository by repository, taken at the pin.
: > "$scratch/expected"
while IFS=$'\t' read -r name path tip source; do
  history="$(git -C "$path" log --reverse --format='%H' refs/heads/calibration)"
  commits="$(printf '%s\n' "$history" | grep -c . || true)"
  install="$(git -C "$path" log --reverse --format='%H' -S"$marker" refs/heads/calibration | head -1 || true)"

  since=0
  trailers_since=0
  trailers_before=0
  seen=0
  for sha in $history; do
    [ -n "$install" ] && [ "$sha" = "$install" ] && seen=1
    allowances="$(git -C "$path" log -1 --format=%B "$sha" | grep -c '^[[:space:]]*Weed-allow:' || true)"
    if [ "$seen" = "1" ]; then
      since=$((since + 1))
      trailers_since=$((trailers_since + allowances))
    else
      trailers_before=$((trailers_before + allowances))
    fi
  done
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$name" "$commits" "$install" "$since" "$trailers_since" "$trailers_before" "$tip" \
    >> "$scratch/expected"
done < "$scratch/corpus"

if ! cargo xtask suppressions --format json > "$scratch/measured.json"; then
  echo "cargo xtask suppressions failed" >&2
  exit 1
fi

python3 - "$scratch/measured.json" "$scratch/expected" "$report" <<'PY' || status=1
import json
import re
import sys

measured = {rate["repo"]: rate for rate in json.load(open(sys.argv[1], encoding="utf-8"))}
report = open(sys.argv[3], encoding="utf-8").read()
complaints = []

expected = []
for line in open(sys.argv[2], encoding="utf-8").read().splitlines():
    name, commits, install, since, trailers_since, trailers_before, tip = line.split("\t")
    expected.append(
        {
            "repo": name,
            "commits": int(commits),
            "install": install or None,
            "commits_since_install": int(since),
            "trailers_since_install": int(trailers_since),
            "trailers_before_install": int(trailers_before),
            "tip": tip,
        }
    )

for want in expected:
    got = measured.get(want["repo"])
    if got is None:
        complaints.append(f"{want['repo']} is in the corpus and the measurement says nothing about it")
        continue
    for field in ("commits", "commits_since_install", "trailers_since_install", "trailers_before_install"):
        if got[field] != want[field]:
            complaints.append(
                f"{want['repo']}: the measurement says {field}={got[field]} and git says {want[field]}"
            )
    if got.get("tip") != want["tip"]:
        complaints.append(
            f"{want['repo']}: the measurement was taken at {got.get('tip')} and the corpus pins {want['tip']}"
        )
    installed = (got.get("installed") or {}).get("sha")
    if installed != want["install"]:
        complaints.append(
            f"{want['repo']}: the measurement puts the guard install at {installed} and git puts it at {want['install']}"
        )
    rate = 0.0 if want["commits_since_install"] == 0 else (
        want["trailers_since_install"] * 100.0 / want["commits_since_install"]
    )
    if abs(got["per_hundred_commits"] - rate) > 1e-9:
        complaints.append(
            f"{want['repo']}: the rate printed is {got['per_hundred_commits']} and the counts give {rate}"
        )
    if want["install"] is None and got["trailers_since_install"] != 0:
        complaints.append(
            f"{want['repo']}: guard was never installed and the rate counts {got['trailers_since_install']} trailers"
        )

# The calibration file carries the rate beside precision, with the true-positive
# count on the same line.
table = re.search(r"^## Precision and the allowance rate$(.*?)^## ", report, re.M | re.S)
if table is None:
    complaints.append("the calibration file has no precision section for the rate to sit beside")
else:
    for want in expected:
        row = re.search(
            rf"^\| {re.escape(want['repo'])} \| ([^|]+) \| (\d+) \| ([^|]+) \|$",
            table.group(1),
            re.M,
        )
        if row is None:
            complaints.append(f"{want['repo']} has no line in the precision table")
            continue
        precision, true_positives, rate = row.group(1).strip(), row.group(2), row.group(3).strip()
        if not re.fullmatch(r"(\d+\.\d%|no blocks)", precision):
            complaints.append(f"{want['repo']}: '{precision}' is not a precision")
        got = measured.get(want["repo"], {})
        printed = re.match(r"([\d.]+) per 100 commits", rate)
        if printed is None:
            complaints.append(f"{want['repo']}: '{rate}' does not read as an allowance rate")
        elif abs(float(printed.group(1)) - got.get("per_hundred_commits", -1)) > 0.05:
            complaints.append(
                f"{want['repo']}: the file says {printed.group(1)} per hundred and the measurement says "
                f"{got.get('per_hundred_commits')}"
            )
        if want["install"] is None and "guard not installed" not in rate:
            complaints.append(
                f"{want['repo']}: guard was never installed there and the line does not say so"
            )
        if true_positives == "":
            complaints.append(f"{want['repo']}: the line carries no true-positive count")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"{len(expected)} repositories, every rate and every install day the same as git's own at the pin")
PY

# The history no garden repository has yet: an install with allowances on both
# sides of it.
cargo test --package xtask --test suppressions || status=1

[ "$status" -eq 0 ] || exit "$status"
echo "cargo xtask suppressions: the rate is git's own count at the pin from the day guard is installed, zero before, and the calibration file carries it beside precision"
