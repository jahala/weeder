#!/usr/bin/env bash
# Evidence for recall.tend2.html c2: recall for the rules that block — T1, T3,
# S1, X1, C1 and G1 — is at or above 95 percent in each of ts, py, rs and go,
# and every warn-level rule's recall is reported beside them.
#
# The campaign is run here rather than read, for the same reason c1 runs it: a
# bar checked against a number somebody typed is not a bar. What the file is
# then held to is arithmetic and coverage — every recall is its own hits over
# its own cases, every rule the binary carries has a figure in every language,
# and a rule that blocks has real cases behind its figure rather than a dash.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the report cannot be read. The check refuses to pass on unread numbers." >&2
  exit 3
}
command -v cargo >/dev/null 2>&1 || {
  echo "cargo is not on PATH, so the campaign cannot be run." >&2
  exit 3
}

cases="${WEED_RECALL_CASES:-40}"
commits="${WEED_RECALL_COMMITS:-200}"
report="docs/calibration-2026-09.md"
run="${WEED_RECALL_CACHE:-${TMPDIR:-/tmp}/weed-recall-corpus}"
mkdir -p "$run"

if ! cargo xtask mutate \
  --cases "$cases" \
  --commits "$commits" \
  --out "$report" \
  --json "$run/bar.json"; then
  echo "the campaign could not run, so there is no recall to hold to a bar." >&2
  exit 3
fi

python3 - "$report" "$run/bar.json" "$cases" <<'PY'
import json
import re
import subprocess
import sys

report_path, cases_path, asked = sys.argv[1], sys.argv[2], int(sys.argv[3])
report = open(report_path, encoding="utf-8").read()
cases = json.load(open(cases_path, encoding="utf-8"))["cases"]
complaints = []

BAR = 95.0
AIM = 80.0
BLOCKING = ["T1", "T3", "S1", "X1", "C1", "G1"]
LANGUAGES = ["ts", "py", "rs", "go"]

catalogue = json.loads(
    subprocess.run(
        ["./target/release/weed", "rules", "--format", "json"],
        capture_output=True, text=True, check=True,
    ).stdout
)
warning_rules = [
    rule["id"] for rule in catalogue if rule["face"] == "check" and rule["level"] == "warn"
]

section = re.search(r"<!-- recall:begin -->(.*?)<!-- recall:end -->", report, re.S)
if section is None:
    print(f"{report_path} carries no recall section", file=sys.stderr)
    raise SystemExit(1)
section = section.group(1)

rows = {}
for line in section.splitlines():
    found = re.match(
        r"\|\s*([A-Z]\d)\s*\|\s*(\w+)\s*\|\s*(\w+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*([^|]+)\|",
        line,
    )
    if found:
        rule, level, language, counted, hits, misses, unplantable, recall = found.groups()
        rows[(rule, language)] = {
            "level": level,
            "cases": int(counted),
            "hits": int(hits),
            "misses": int(misses),
            "unplantable": int(unplantable),
            "recall": recall.strip(),
        }

# The arithmetic, recomputed. A recall is hits over cases and nothing else.
for (rule, language), row in rows.items():
    if row["cases"] != row["hits"] + row["misses"]:
        complaints.append(
            f"{rule} in {language}: {row['cases']} cases is not {row['hits']} hits "
            f"and {row['misses']} misses"
        )
    if row["cases"] == 0:
        if row["recall"] != "no cases":
            complaints.append(f"{rule} in {language}: no cases, and a recall of {row['recall']}")
        continue
    measured = row["hits"] * 100.0 / row["cases"]
    written = row["recall"].rstrip("%")
    try:
        if abs(float(written) - measured) > 0.05:
            complaints.append(
                f"{rule} in {language}: the file says {row['recall']}, the counts make {measured:.1f}%"
            )
    except ValueError:
        complaints.append(f"{rule} in {language}: '{row['recall']}' is not a recall")

# The same numbers the run came back with.
counted = {}
for case in cases:
    key = (case["rule"], case["language"])
    hits, total = counted.get(key, (0, 0))
    counted[key] = (hits + (1 if case["caught"] else 0), total + 1)
for key, (hits, total) in counted.items():
    row = rows.get(key)
    if row is None:
        complaints.append(f"{key[0]} in {key[1]}: the run made cases the file has no row for")
    elif (row["cases"], row["hits"]) != (total, hits):
        complaints.append(
            f"{key[0]} in {key[1]}: the run counted {total}/{hits}, the file says "
            f"{row['cases']}/{row['hits']}"
        )

# The bar: every rule that blocks, in every language, on cases that exist.
for rule in BLOCKING:
    for language in LANGUAGES:
        row = rows.get((rule, language))
        if row is None:
            complaints.append(f"{rule} in {language}: the file carries no row for a rule that blocks")
            continue
        if row["level"] != "block":
            complaints.append(f"{rule} is reported as {row['level']}, and the bar is for what blocks")
        if row["cases"] == 0:
            complaints.append(
                f"{rule} in {language}: no cases at all, so nothing was measured to hold to the bar"
            )
            continue
        measured = row["hits"] * 100.0 / row["cases"]
        if measured < BAR:
            complaints.append(
                f"{rule} in {language}: {measured:.1f}% is under the {BAR:.0f}% bar "
                f"({row['misses']} of {row['cases']} missed)"
            )

# Every warn-level rule is reported, in every language: a figure, or a reason
# there is none. Warn rules aim at 80 percent, and one under it is named here
# rather than being allowed to pass unmentioned.
under = []
for rule in warning_rules:
    for language in LANGUAGES:
        row = rows.get((rule, language))
        if row is None:
            complaints.append(f"{rule} in {language}: no recall is reported")
            continue
        if row["cases"] == 0:
            if not re.search(rf"^- {rule} · {language}, .+$", section, re.M):
                complaints.append(
                    f"{rule} in {language}: no cases and no reason given for having none"
                )
            continue
        measured = row["hits"] * 100.0 / row["cases"]
        if measured < AIM:
            under.append(f"{rule} in {language} at {measured:.1f}%")

# Every miss the run made is written down: a bar met by leaving misses out of
# the file would be the worst way to pass this.
missed = sum(1 for case in cases if not case["caught"])
named = len(re.findall(r"^- [A-Z]\d · \w+ · [\w.-]+ `[0-9a-f]{9}`", section, re.M))
if named != missed:
    complaints.append(f"the run missed {missed} cases and the file names {named}")

# A bar held up by a handful of cases is not one. The campaign's own size is
# part of what makes the number mean anything.
FLOOR = 1000
if asked < 20:
    complaints.append(f"the run asked for {asked} cases a rule, too few to hold a rule to a bar")
if len(cases) < FLOOR:
    complaints.append(f"the run planted {len(cases)} cases, and the bar wants at least {FLOOR}")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)

worst = min(
    (row["hits"] * 100.0 / row["cases"], f"{rule} in {language}")
    for (rule, language), row in rows.items()
    if rule in BLOCKING and row["cases"] > 0
)
print(
    f"every rule that blocks is at or above {BAR:.0f}% in all four languages "
    f"(worst: {worst[1]} at {worst[0]:.1f}%); "
    f"{len(warning_rules)} warn-level rules reported"
    + (f", {len(under)} of them under the {AIM:.0f}% aim: " + ", ".join(under) if under else "")
)
PY
