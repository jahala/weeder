#!/usr/bin/env bash
# Evidence for calibration: the report prints, permanently and dated, the
# false-positive count and share under the classes in use before the ruling of
# 2026-09-06 beside the count and share under it, read from the record the day
# was written down in, and the after side of that record is what the ledger
# counts today.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
report="docs/calibration-2026-09.md"
record="docs/calibration/ruling-2026-09-06.toml"
[ -f "$record" ] || { echo "$record is missing: the day the question changed is not written down" >&2; exit 1; }
[ -f "$report" ] || { echo "$report is missing" >&2; exit 1; }
python3 - "$report" "$record" <<'PY'
import re, sys
report = open(sys.argv[1], encoding="utf-8").read()
record = open(sys.argv[2], encoding="utf-8").read()
def field(section, key):
    block = re.search(rf"^\[{section}\]\n(.*?)(?=^\[|\Z)", record, re.M | re.S)
    if not block:
        raise SystemExit(f"{sys.argv[2]} has no [{section}] table")
    value = re.search(rf"^{key}\s*=\s*(.+)$", block.group(1), re.M)
    if not value:
        raise SystemExit(f"{sys.argv[2]} [{section}] has no {key}")
    return value.group(1).strip().strip('"')
date = re.search(r'^date\s*=\s*"([^"]+)"', record, re.M).group(1)
before_count, before_share = field("before", "false_positives"), float(field("before", "share_percent"))
after_count, after_share = field("after", "false_positives"), float(field("after", "share_percent"))
complaints = []
# The verdict sentence carries the count and share the ledger gives today, and
# that is the record's after side, or the record is stale.
verdict = re.search(r"it blocked (\d+), of which (\d+) were block-level false positives, ([0-9.]+) percent", report)
if not verdict:
    complaints.append("the verdict sentence does not carry a blocked count, a false-positive count and a share")
else:
    if verdict.group(2) != after_count:
        complaints.append(f"the ledger counts {verdict.group(2)} false positives today and the record's after side says {after_count}")
    if abs(float(verdict.group(3)) - after_share) > 0.005:
        complaints.append(f"the ledger's share is {verdict.group(3)} today and the record's after side says {after_share:.2f}")
# Both numbers, with the date, in one sentence a reader meets before any table.
first_part = report.split("\n## ", 1)[0]
wanted = [date, f"{before_count} false positives", f"{before_share:.2f} percent", f"{after_count}", f"{after_share:.2f} percent"]
for piece in wanted:
    if piece not in first_part:
        complaints.append(f"the report's opening does not carry `{piece}` beside the ruling's date")
if "under the three classes" not in first_part.lower() and "before the ruling" not in first_part.lower():
    complaints.append("the report's opening does not say which number belongs to the classes before the ruling")
for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"the report carries both sides of the {date} ruling: {before_count} false positives, {before_share:.2f} percent before it, {after_count}, {after_share:.2f} percent under it")
PY
