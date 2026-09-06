p = "scripts/check/calibration-bar.sh"
s = open(p).read()

old = """# The kill bar is read as a fact rather than as a promise. The file says what
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
result="docs/calibration/metric.json\""""
new = """# The kill bar is read as a fact rather than as a promise. The file says what
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
audit="${WEED_CALIBRATION_AUDIT:-docs/calibration-audit-2026-09.md}"
result="${WEED_CALIBRATION_METRIC:-docs/calibration/metric.json}\""""
assert old in s
s = s.replace(old, new)

old = """python3 - "$report" "$corpus" "$result" <<'PY'
import json
import re
import sys

report_path, corpus_path, result_path = sys.argv[1], sys.argv[2], sys.argv[3]
report = open(report_path, encoding="utf-8").read()
complaints = []

BAR = 2.0
LOAD_BEARING = ("T1", "T2", "T3", "S1")
CLASSES = ("true positive", "acceptable", "false positive", "unclassified")"""
new = """python3 - "$report" "$corpus" "$result" "$audit" <<'PY'
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
CONFIRMED = "weed ships as a gate:\""""
assert old in s
s = s.replace(old, new)

old = """verdict = next((line for line in report.splitlines()[1:] if line.strip()), "")
ships = verdict.startswith("weed ships as a gate:")
if ships and complaints:
    complaints.append("the file claims weed ships as a gate, and the numbers under it do not agree")
if not ships and not complaints:
    complaints.append(
        "the numbers clear the bar and the file's first sentence does not say weed ships as a gate"
    )"""
new = """# The re-grade, read the same way the generator reads it: a row per sample with
# how many cases were re-graded and how many agreed, and the share recomputed
# from those two rather than taken from the column beside them.
def regrade(path):
    try:
        text = open(path, encoding="utf-8").read()
    except OSError:
        return None
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
    )"""
assert old in s
s = s.replace(old, new)

old = """        "share_percent": share,
        "verdict": "ship","""
new = """        "share_percent": share,
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
        ],"""
assert old in s
s = s.replace(old, new)

old = """print(
    f"{share}% of {pooled['judged']} commits judged are block-level false positives, "
    f"under the {BAR}% bar, with {' '.join(LOAD_BEARING)} all at block level"
)"""
new = """print(
    f"{share}% of {pooled['judged']} commits judged are block-level false positives, "
    f"under the {BAR}% bar, with {' '.join(LOAD_BEARING)} all at block level, and the verdict "
    f"{'stands on the re-grade' if confirmed else 'reads as pending until the re-grade lands'}"
)"""
assert old in s
s = s.replace(old, new)

open(p, "w").write(s)
print("ok")
