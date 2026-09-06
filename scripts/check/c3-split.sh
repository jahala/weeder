#!/usr/bin/env bash
# Evidence for calibration c7: beside the pooled block-level false-positive share
# the report carries the count of C3 warnings on workflow files that C1 blocked
# in the first run, and that count is the table under it added up rather than a
# number somebody typed.
#
# Splitting a rule in two is meant to take friction off the gate without taking
# the finding away, and until the same files are read twice that is an opinion.
# So the first run's block-level findings are kept in
# docs/calibration/first-run.toml, one per file per commit, and this run says
# what it makes of each of them. Three numbers have to agree here: the paragraph
# beside the pooled share, the rows of the table under it, and the record the
# comparison was made against. Nothing is believed because the report says it.
#
# The suite in xtask/tests/split.rs runs last, on a history built for the
# purpose, for the cases this corpus does not happen to contain: a finding the
# same rule still refuses, a finding nothing reports any more, and a finding
# sitting on a commit outside the window.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

report="docs/calibration-2026-09.md"
record="docs/calibration/first-run.toml"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the report cannot be added up. The check refuses to pass on unchecked arithmetic." >&2
  exit 3
}
[ -f "$report" ] || { echo "$report is missing: there is no report to read the split off" >&2; exit 1; }
[ -f "$record" ] || {
  echo "$record is missing: without what the first run refused there is nothing to measure the split against" >&2
  exit 1
}

status=0

python3 - "$report" "$record" <<'PY' || status=1
import re
import sys
from collections import Counter

report = open(sys.argv[1], encoding="utf-8").read()
record = open(sys.argv[2], encoding="utf-8").read()
complaints = []

# What the first run refused, read out of the record the report names.
recorded = []
for block in record.split("[[block]]")[1:]:
    fields = {}
    for key in ("repo", "commit", "rule", "path"):
        found = re.search(rf'^\s*{key}\s*=\s*"([^"]*)"', block, re.M)
        if found:
            fields[key] = found.group(1)
    if len(fields) == 4:
        recorded.append(fields)
if not recorded:
    complaints.append("the record of the first run carries no finding, so there is nothing to compare")

# The paragraph beside the pooled share.
totals = re.search(r"^## Totals$(.*?)^## ", report, re.M | re.S)
if totals is None:
    complaints.append("the report has no totals section for the split to sit beside")
    paragraph = ""
else:
    paragraph = next(
        (
            line
            for line in totals.group(1).splitlines()
            if line.startswith("Beside that share, what the rules moved.")
        ),
        "",
    )
    if not paragraph:
        complaints.append(
            "the pooled share carries nothing beside it about what the rules moved, so the split's "
            "effect is assumed rather than measured"
        )

stated = re.search(
    r"The first run refused (\d+) findings at block level on these commits, recorded in `([^`]+)`\. "
    r"This run reports (\d+) of them at block level still, (\d+) at warn level, (\d+) at note level "
    r"and (\d+) not at all\.",
    paragraph,
)
if paragraph and stated is None:
    complaints.append(f"the paragraph does not read as a count of what moved: {paragraph!r}")

# The table under it, counted row by row.
section = re.search(
    r"^## What the rules moved since the first run$(.*?)(?=^## |\Z)", report, re.M | re.S
)
rows = []
if section is None:
    complaints.append("the counts have no table under them, so nobody can add them up")
    left_out = None
else:
    for row in re.finditer(
        r"^\| (\S+) \| `([0-9a-f]{7,})` \| `([^`]+)` \| (\w+) \| ([A-Z]\d+) \| ([A-Z]\d+|nothing) \| (\w+) \|$",
        section.group(1),
        re.M,
    ):
        rows.append(
            {
                "repo": row.group(1),
                "commit": row.group(2),
                "path": row.group(3),
                "kind": row.group(4),
                "then": row.group(5),
                "now": row.group(6),
                "level": row.group(7),
            }
        )
    counted = re.search(
        r"^(\d+) of the first run's block-level findings are refused by the same rule at the same "
        r"level and are left out of this table; the (\d+) whose answer changed are all in it\.$",
        section.group(1),
        re.M,
    )
    left_out = counted
    if counted is None:
        complaints.append("the table does not say how many findings it leaves out and how many it holds")
    else:
        if int(counted.group(2)) != len(rows):
            complaints.append(
                f"the table says it holds {counted.group(2)} rows and it carries {len(rows)}"
            )

# The three have to agree.
if stated is not None and left_out is not None:
    total = int(stated.group(1))
    unchanged = int(left_out.group(1))
    if total != unchanged + len(rows):
        complaints.append(
            f"the paragraph counts {total} findings, and the table holds {len(rows)} rows beside "
            f"{unchanged} it leaves out"
        )
    if total != len(recorded):
        complaints.append(
            f"the paragraph counts {total} findings and {sys.argv[2]} records {len(recorded)}"
        )
    if stated.group(2) != sys.argv[2]:
        complaints.append(
            f"the paragraph says it compared with {stated.group(2)} and this check read {sys.argv[2]}"
        )
    at = {"block": 0, "warn": 0, "note": 0, "nothing": 0}
    for row in rows:
        at[row["level"]] = at.get(row["level"], 0) + 1
    at["block"] += unchanged
    for level, printed in (
        ("block", int(stated.group(3))),
        ("warn", int(stated.group(4))),
        ("note", int(stated.group(5))),
        ("nothing", int(stated.group(6))),
    ):
        if at.get(level, 0) != printed:
            complaints.append(
                f"the paragraph says {printed} findings are at {level} now and the table counts "
                f"{at.get(level, 0)}"
            )

# The C1 split itself: every pair the paragraph names, recomputed from the rows.
moves = Counter(
    (row["then"], row["now"], row["kind"]) for row in rows if row["level"] == "warn"
)
for named in re.finditer(
    r"(\d+) moved from ([A-Z]\d+) at block level to ([A-Z]\d+) at warn level, all of them on (\w+) files",
    paragraph,
):
    count, then, now, kind = int(named.group(1)), named.group(2), named.group(3), named.group(4)
    if moves.get((then, now, kind), 0) != count:
        complaints.append(
            f"the paragraph says {count} findings moved from {then} to {now} on {kind} files and the "
            f"table carries {moves.get((then, now, kind), 0)}"
        )
counted_pairs = sum(moves.values())
paragraph_pairs = sum(
    int(found.group(1))
    for found in re.finditer(r"(\d+) moved from [A-Z]\d+ at block level to [A-Z]\d+ at warn level", paragraph)
)
if rows and counted_pairs != paragraph_pairs:
    complaints.append(
        f"the table carries {counted_pairs} findings that moved to warn level and the paragraph "
        f"accounts for {paragraph_pairs}"
    )

# Every row has to be one of the first run's findings, and no finding of the
# first run may go missing from the comparison.
by_key = {(entry["repo"], entry["commit"][:10], entry["path"]) for entry in recorded}
for row in rows:
    if (row["repo"], row["commit"], row["path"]) not in by_key:
        complaints.append(
            f"{row['repo']} {row['commit']} {row['path']} is in the table and not in the first run's record"
        )
outside = re.search(
    r"(\d+) of the first run's block-level findings sit on commits outside this window", paragraph
)
if outside and stated and int(outside.group(1)) + int(stated.group(1)) != len(recorded):
    complaints.append(
        "the findings compared and the findings outside the window do not add up to the record"
    )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)

workflow = sum(
    count
    for (then, now, kind), count in moves.items()
    if kind == "workflow"
)
print(
    f"{len(recorded)} findings the first run refused, {len(rows)} of which this run answers "
    f"differently, {workflow} of them warnings on workflow files that used to block"
)
PY

# The cases this corpus does not happen to contain.
cargo test --package xtask --test split || status=1

[ "$status" -eq 0 ] || exit "$status"
echo "the report measures what the rule split moved against the first run's own findings, and the count beside the pooled share is the table under it added up"
