#!/usr/bin/env bash
# Evidence for recall.tend2.html c1: `cargo xtask mutate` injects one anti-pattern
# per case for every check rule in each of ts, py, rs and go, from the same
# corpus calibration measures precision on, runs weeder on each case, and writes
# the recall section of docs/calibration-2026-09.md with every miss named.
#
# The campaign is run here rather than read from the file, so this check can
# never pass on numbers somebody typed. What is then checked is the shape of
# what came back: that every rule was tried in every language, that a cell with
# no cases says why, that every miss the run recorded is named in the file by
# repository, commit and site, and that each case carries one rule and one site,
# which is what makes a hit attributable.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the run cannot be read back. The check refuses to pass on an unread report." >&2
  exit 3
}
command -v cargo >/dev/null 2>&1 || {
  echo "cargo is not on PATH, so the campaign cannot be run." >&2
  exit 3
}

cases="${WEEDER_RECALL_CASES:-40}"
commits="${WEEDER_RECALL_COMMITS:-200}"
report="docs/calibration-2026-09.md"
# The run's own numbers land beside the cases the campaign keeps, under one
# known path that the next run writes over: nothing here deletes anything.
run="${WEEDER_RECALL_CACHE:-${TMPDIR:-/tmp}/weeder-recall-corpus}"
mkdir -p "$run"

# The injector's own scanner is what makes the measurement independent of weeder,
# so it is held up by its own tests before it is trusted to find a site.
cargo test --release --quiet -p xtask

# A corpus that cannot be reached is a measurement that could not be made, and
# that leaves with 3 rather than looking like a clean one.
if ! cargo xtask mutate \
  --cases "$cases" \
  --commits "$commits" \
  --out "$report" \
  --json "$run/cases.json"; then
  echo "the campaign could not run. every repository of the corpus has to be readable; name one somewhere else with WEEDER_CORPUS_<NAME>." >&2
  exit 3
fi

python3 - "$report" "$run/cases.json" "$cases" <<'PY'
import json
import re
import subprocess
import sys

report_path, cases_path, asked = sys.argv[1], sys.argv[2], int(sys.argv[3])
report = open(report_path, encoding="utf-8").read()
run = json.load(open(cases_path, encoding="utf-8"))
cases = run["cases"]
complaints = []

LANGUAGES = ["ts", "py", "rs", "go"]

# The rules are read off the binary, so a rule that lands tomorrow is one this
# check starts asking about tomorrow.
catalogue = json.loads(
    subprocess.run(
        ["./target/release/weeder", "rules", "--format", "json"],
        capture_output=True, text=True, check=True,
    ).stdout
)
check_rules = [rule["id"] for rule in catalogue if rule["face"] == "check"]

# The section, and the table inside it.
section = re.search(r"<!-- recall:begin -->(.*?)<!-- recall:end -->", report, re.S)
if section is None:
    complaints.append(f"{report_path} carries no recall section")
    print("\n".join(complaints), file=sys.stderr)
    raise SystemExit(1)
section = section.group(1)

rows = {}
for line in section.splitlines():
    found = re.match(
        r"\|\s*([A-Z]\d)\s*\|\s*(\w+)\s*\|\s*(\w+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|",
        line,
    )
    if found:
        rule, _, language, counted, hits, misses = found.groups()
        rows[(rule, language)] = (int(counted), int(hits), int(misses))

# Every rule, in every language, with a cell that says what happened.
for rule in check_rules:
    for language in LANGUAGES:
        if (rule, language) not in rows:
            complaints.append(f"{report_path} has no row for {rule} in {language}")

# What the run recorded is what the file says.
counted = {}
for case in cases:
    key = (case["rule"], case["language"])
    hit, total = counted.get(key, (0, 0))
    counted[key] = (hit + (1 if case["caught"] else 0), total + 1)
for key, (hits, total) in counted.items():
    written = rows.get(key)
    if written is None:
        continue
    if written[:2] != (total, hits):
        complaints.append(
            f"{key[0]} in {key[1]}: the run counted {total} cases and {hits} hits, "
            f"the file says {written[0]} and {written[1]}"
        )

# A cell with no cases is accounted for rather than left blank.
for rule in check_rules:
    for language in LANGUAGES:
        if counted.get((rule, language), (0, 0))[1] > 0:
            continue
        if not re.search(rf"^- {rule} · {language}, .+$", section, re.M):
            complaints.append(
                f"{rule} in {language} has no cases and the file gives no reason"
            )

# Every miss is named, by repository, commit and site.
for case in cases:
    if case["caught"]:
        continue
    short = case["commit"][:9]
    named = [
        line
        for line in section.splitlines()
        if line.startswith(f"- {case['rule']} · {case['language']} ·")
        and short in line
        and case["path"] in line
    ]
    if not named:
        complaints.append(
            f"the miss {case['rule']}/{case['language']} at {case['repository']} "
            f"{short} {case['path']} is not named in the file"
        )

# One rule, one site, per case: a case that named two of either would make a hit
# unattributable, which is the one thing this campaign may not do.
for case in cases:
    if not case["rule"] or not case["language"] or not case["path"]:
        complaints.append(f"a case carries no rule, language or site: {case}")
    if not case["shape"]:
        complaints.append(f"a case says nothing about what was planted: {case}")

# A campaign small enough to prove nothing is one nobody should be able to pass
# this check with.
FLOOR = 1000
if asked < 20:
    complaints.append(f"the run asked for {asked} cases a rule, which is too few to measure recall")
if len(cases) < FLOOR:
    complaints.append(f"the run planted {len(cases)} cases, and the check wants at least {FLOOR}")
repositories = run["repositories"]
if len(repositories) < 5:
    complaints.append(f"the corpus was {len(repositories)} repositories; calibration reads five")
for repository in repositories:
    if not re.fullmatch(r"[0-9a-f]{40}", repository["head"]):
        complaints.append(f"{repository['repository']} names no commit it was read at")
    if repository["commits"] == 0:
        complaints.append(f"{repository['repository']} contributed no commit")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)

languages = {case["language"] for case in cases}
print(
    f"{len(cases)} cases planted one anti-pattern at a time across "
    f"{len(repositories)} repositories in {len(languages)} languages, "
    f"{sum(1 for case in cases if not case['caught'])} missed and every one of them named in {report_path}"
)
PY
