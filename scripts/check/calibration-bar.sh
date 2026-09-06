#!/usr/bin/env bash
# Evidence for calibration c2, and the measurement garden.json names: the pooled
# block-level false-positive share is under two percent of the commits judged,
# every repository's own count and share is in the file, and the bar was not
# reached by turning a load-bearing rule down.
#
# Nothing here trusts a number the report printed. Every per-repository table is
# counted row by row, the totals are recomputed from those counts, each share is
# recomputed from its own two numbers, and the pooled row is recomputed from the
# repositories. A report whose arithmetic disagrees with its own tables is
# refused before the bar is even read. The judgement itself is never touched:
# whether a block was a false positive is a person's reading, written in the
# ledger, and this script only adds up what they wrote.
#
# The kill bar is read as a fact rather than as a promise. The file says what
# level every rule ran at, and T1, T2, T3 and S1 have to say block; and no line
# anywhere in it may record one of those four as turned off.
#
# The result is written to docs/calibration/metric.json, which is the metric's
# latest answer and is committed beside the report it was read from.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

report="docs/calibration-2026-09.md"
corpus="docs/calibration/corpus.toml"
result="docs/calibration/metric.json"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the report cannot be added up. The check refuses to pass on unchecked arithmetic." >&2
  exit 3
}
[ -f "$report" ] || { echo "$report is missing: there is no calibration to read a bar off" >&2; exit 1; }

python3 - "$report" "$corpus" "$result" <<'PY'
import json
import re
import sys

report_path, corpus_path, result_path = sys.argv[1], sys.argv[2], sys.argv[3]
report = open(report_path, encoding="utf-8").read()
complaints = []

BAR = 2.0
LOAD_BEARING = ("T1", "T2", "T3", "S1")
CLASSES = ("true positive", "acceptable", "false positive", "unclassified")


def rounded(part, whole):
    """The share, to the two decimals the report writes."""
    return 0.0 if whole == 0 else round(part * 100.0 / whole, 2)


# The repositories the file has to account for.
named = re.findall(r'^\s*name\s*=\s*"([^"]+)"', open(corpus_path, encoding="utf-8").read(), re.M)

# Each repository's section: the heading's counts, and the classification of
# every blocked commit under it.
sections = re.split(r"^## ", report, flags=re.M)
counted = {}
for section in sections:
    heading = re.match(r"(\S+) — (\d+) commits judged, (\d+) blocked, (\d+) warned$", section.splitlines()[0] if section else "")
    if heading is None:
        continue
    repo, judged, blocked, warned = heading.group(1), int(heading.group(2)), int(heading.group(3)), int(heading.group(4))
    classes = {name: 0 for name in CLASSES}
    rows = 0
    for row in re.finditer(r"^\| `[0-9a-f]{7,}`[^|]*\|[^|]*\| ([^|]+) \|[^|]*\|$", section, re.M):
        classification = row.group(1).strip()
        if classification not in classes:
            complaints.append(f"{repo}: '{classification}' is not one of {CLASSES}")
            continue
        classes[classification] += 1
        rows += 1
    if rows != blocked:
        complaints.append(
            f"{repo}: the heading says {blocked} blocked and the table carries {rows} rows"
        )
    counted[repo] = {
        "judged": judged,
        "blocked": blocked,
        "warned": warned,
        "true_positive": classes["true positive"],
        "acceptable": classes["acceptable"],
        # A block nobody has classified is a false positive until somebody reads it.
        "false_positive": classes["false positive"] + classes["unclassified"],
        "unclassified": classes["unclassified"],
    }

for repo in named:
    if repo not in counted:
        complaints.append(f"{repo} is in the corpus and has no section in the report")

# The totals table, recomputed from the sections rather than read.
totals = {}
pooled_row = None
for row in re.finditer(
    r"^\| (\*\*)?([\w-]+)\1? \| (\*\*)?(\d+)\3? \| (\*\*)?(\d+)\5? \| (\*\*)?(\d+)\7? \| (\*\*)?(\d+)\9? \| (\*\*)?(\d+)\11? \| (\*\*)?(\d+)\13? \| (\*\*)?([\d.]+)%\15? \|$",
    report,
    re.M,
):
    entry = {
        "judged": int(row.group(4)),
        "blocked": int(row.group(6)),
        "warned": int(row.group(8)),
        "true_positive": int(row.group(10)),
        "acceptable": int(row.group(12)),
        "false_positive": int(row.group(14)),
        "share": float(row.group(16)),
    }
    if row.group(2) == "pooled":
        pooled_row = entry
    else:
        totals[row.group(2)] = entry

for repo, tally in counted.items():
    printed = totals.get(repo)
    if printed is None:
        complaints.append(f"{repo} has a section and no row in the totals: its own count is not shown")
        continue
    for field in ("judged", "blocked", "warned", "true_positive", "acceptable", "false_positive"):
        if printed[field] != tally[field]:
            complaints.append(
                f"{repo}: the totals say {field}={printed[field]} and its own table counts {tally[field]}"
            )
    if printed["share"] != rounded(tally["false_positive"], tally["judged"]):
        complaints.append(
            f"{repo}: the share printed is {printed['share']}% and the numbers give "
            f"{rounded(tally['false_positive'], tally['judged'])}%"
        )

if pooled_row is None:
    complaints.append("the totals carry no pooled row, so there is no pooled share to read")

pooled = {
    field: sum(tally[field] for tally in counted.values())
    for field in ("judged", "blocked", "warned", "true_positive", "acceptable", "false_positive")
}
share = rounded(pooled["false_positive"], pooled["judged"])
if pooled_row is not None:
    for field in ("judged", "blocked", "warned", "true_positive", "acceptable", "false_positive"):
        if pooled_row[field] != pooled[field]:
            complaints.append(
                f"the pooled row says {field}={pooled_row[field]} and the repositories add up to {pooled[field]}"
            )
    if pooled_row["share"] != share:
        complaints.append(
            f"the pooled share printed is {pooled_row['share']}% and the numbers give {share}%"
        )

if pooled["judged"] == 0:
    complaints.append("no commits were judged, so there is no share to put against the bar")
elif share >= BAR:
    complaints.append(
        f"the pooled block-level false-positive share is {share}% of {pooled['judged']} commits, "
        f"at or over the {BAR}% bar. weed does not ship as a gate on this measurement."
    )

# The kill bar: the four rules it names ran at block level, and no line of the
# file records one of them turned off.
levels = dict(re.findall(r"^\| ([A-Z]\d+) \| (block|warn|note|not run) \|", report, re.M))
for rule in LOAD_BEARING:
    if rule not in levels:
        complaints.append(f"the report never says what level {rule} ran at")
    elif levels[rule] != "block":
        complaints.append(
            f"{rule} ran at {levels[rule]}, not block: the bar was reached with a rule the kill bar names turned down"
        )
off = re.compile(
    r"\b(?:" + "|".join(LOAD_BEARING) + r")\b[^\n]{0,80}?\b(?:turned off|switched off|disabled|off\b)",
    re.I,
)
for number, line in enumerate(report.splitlines(), start=1):
    if off.search(line):
        complaints.append(f"line {number} records a load-bearing rule as off: {line.strip()!r}")

verdict = next((line for line in report.splitlines()[1:] if line.strip()), "")
ships = verdict.startswith("weed ships as a gate:")
if ships and complaints:
    complaints.append("the file claims weed ships as a gate, and the numbers under it do not agree")
if not ships and not complaints:
    complaints.append(
        "the numbers clear the bar and the file's first sentence does not say weed ships as a gate"
    )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)

json.dump(
    {
        "metric": "block-level false-positive share across the calibration repositories",
        "source": report_path,
        "bar_percent": BAR,
        "commits_judged": pooled["judged"],
        "blocked": pooled["blocked"],
        "false_positives": pooled["false_positive"],
        "share_percent": share,
        "verdict": "ship",
        "repos": [
            {
                "repo": repo,
                "commits_judged": tally["judged"],
                "blocked": tally["blocked"],
                "false_positives": tally["false_positive"],
                "share_percent": rounded(tally["false_positive"], tally["judged"]),
            }
            for repo, tally in sorted(counted.items())
        ],
    },
    open(result_path, "w", encoding="utf-8"),
    indent=2,
    sort_keys=True,
)
open(result_path, "a", encoding="utf-8").write("\n")
print(
    f"{share}% of {pooled['judged']} commits judged are block-level false positives, "
    f"under the {BAR}% bar, with {' '.join(LOAD_BEARING)} all at block level"
)
PY
