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
# The verdict's wording is held to the re-grade. Whether a block was a false
# positive is a person's reading, and the person who ran the calibration read
# them all, so until a second party has re-graded a sample and agreed the first
# sentence has to say the verdict is pending. An unqualified `weed ships as a
# gate:` while `docs/calibration-audit-2026-09.md` is missing, or under the bar,
# is refused here.
#
# The result is written to docs/calibration/metric.json, which is the metric's
# latest answer and is committed beside the report it was read from.
#
# The four files are overridable so a suite can run this script against a report
# it wrote itself; with nothing set it reads the ones the repository ships.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

report="${WEED_CALIBRATION_REPORT:-docs/calibration-2026-09.md}"
corpus="${WEED_CALIBRATION_CORPUS:-docs/calibration/corpus.toml}"
# Every audit the repository carries counts, packet, response and transcript
# files aside: a blind re-grade under the bar keeps the verdict pending however
# well a sighted one did. The probes name one file to read instead.
audit="${WEED_CALIBRATION_AUDIT:-$(ls docs/calibration-audit*.md 2>/dev/null | grep -v '\.packet\.\|\.response\.\|transcript' | tr '\n' ':' | sed 's/:$//')}"
result="${WEED_CALIBRATION_METRIC:-docs/calibration/metric.json}"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the report cannot be added up. The check refuses to pass on unchecked arithmetic." >&2
  exit 3
}
[ -f "$report" ] || { echo "$report is missing: there is no calibration to read a bar off" >&2; exit 1; }

python3 - "$report" "$corpus" "$result" "$audit" <<'PY'
import json
import re
import sys

report_path, corpus_path, result_path, audit_path = sys.argv[1:5]
report = open(report_path, encoding="utf-8").read()
complaints = []

BAR = 2.0
AGREEMENT_BAR = 90.0
SAMPLE_FLOOR = 20
LOAD_BEARING = ("T1", "T2", "T3", "S1")
CLASSES = ("true positive", "acceptable", "false positive", "unclassified")
PROVISIONAL = "weed ships as a gate, pending the independent re-grade:"
CONFIRMED = "weed ships as a gate:"


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
    heading = re.match(r"(\S+), (\d+) commits judged, (\d+) blocked, (\d+) warned$", section.splitlines()[0] if section else "")
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

# The re-grade, read the same way the generator reads it: a row per sample with
# how many cases were re-graded and how many agreed, and the share recomputed
# from those two rather than taken from the column beside them.
def regrade(paths):
    """The samples of every audit file named, colon-separated, each sample named
    by its file; None when not one of them is there."""
    found = []
    for path in [one for one in paths.split(":") if one]:
        try:
            found.append((path, open(path, encoding="utf-8").read()))
        except OSError:
            continue
    if not found:
        return None
    samples = []
    for path, text in found:
        samples.extend((f"{name} in {path}", regraded, agreed) for name, regraded, agreed in one_file(text))
    return samples


def one_file(text):
    samples = []
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) < 3:
            continue
        try:
            regraded, agreed = int(cells[1]), int(cells[2])
        except ValueError:
            continue
        name = cells[0].strip("*").strip()
        if name:
            samples.append((name, regraded, agreed))
    return samples


samples = regrade(audit_path)
confirmed = bool(samples) and all(
    regraded >= SAMPLE_FLOOR
    and agreed <= regraded
    and agreed * 100.0 / regraded >= AGREEMENT_BAR
    for _, regraded, agreed in samples
)

verdict = next((line for line in report.splitlines()[1:] if line.strip()), "")
provisional = verdict.startswith(PROVISIONAL)
ships = verdict.startswith(CONFIRMED) or provisional
if ships and complaints:
    complaints.append("the file claims weed ships as a gate, and the numbers under it do not agree")
if not ships and not complaints:
    complaints.append(
        "the numbers clear the bar and the file's first sentence does not say weed ships as a gate"
    )
if ships and not confirmed and not provisional:
    why = (
        f"{audit_path} is not there"
        if samples is None
        else f"{audit_path} draws no sample"
        if not samples
        else "; ".join(
            f"{name} agrees on {agreed * 100.0 / regraded:.1f} percent of {regraded} cases"
            if regraded
            else f"{name} re-graded nothing"
            for name, regraded, agreed in samples
            if not (
                regraded >= SAMPLE_FLOOR
                and agreed <= regraded
                and agreed * 100.0 / regraded >= AGREEMENT_BAR
            )
        )
    )
    complaints.append(
        f"the file says `{CONFIRMED}` with no re-grade behind it: {why}. the verdict stays pending "
        f"until a second party agrees at or above {AGREEMENT_BAR:.0f} percent on every sample of at "
        f"least {SAMPLE_FLOOR} cases."
    )
if ships and confirmed and provisional:
    complaints.append(
        f"the re-grade in {audit_path} agrees at the bar and the first sentence still calls the "
        "verdict pending. the wording is written from that file, so run the calibration again."
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
        "verdict": "ship" if confirmed else "ship, pending the independent re-grade",
        "regrade": None
        if samples is None
        else [
            {
                "sample": name,
                "regraded": regraded,
                "agreed": agreed,
                "agreement_percent": rounded(agreed, regraded),
            }
            for name, regraded, agreed in samples
        ],
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
    f"under the {BAR}% bar, with {' '.join(LOAD_BEARING)} all at block level, and the verdict "
    f"{'stands on the re-grade' if confirmed else 'reads as pending until the re-grade lands'}"
)
PY
