#!/usr/bin/env bash
# Evidence for calibration-audit.tend2.html: the refused blind audit is kept as
# a labelled specimen, and the live agreement evidence does not read that
# specimen path.
set -euo pipefail

python3 - <<'PY'
import pathlib
import re
import subprocess
import sys

SPECIMEN = pathlib.Path("docs/calibration/refused/blind-2026-09-first")
AGREEMENT = pathlib.Path("scripts/check/calibration-agreement.sh")
complaints = []

for name in ("packet.md", "transcript.md", "audit.md", "reason.txt"):
    path = SPECIMEN / name
    if not path.exists():
        complaints.append(f"{path} is missing")
    elif path.stat().st_size == 0:
        complaints.append(f"{path} is empty")

reason = (SPECIMEN / "reason.txt").read_text(encoding="utf-8") if (SPECIMEN / "reason.txt").exists() else ""
if len([line for line in reason.splitlines() if line.strip()]) != 1:
    complaints.append(f"{SPECIMEN / 'reason.txt'} must be exactly one non-empty line")
if "verbatim git hunks" not in reason:
    complaints.append(f"{SPECIMEN / 'reason.txt'} does not say why the audit was refused")

packet = (SPECIMEN / "packet.md").read_text(encoding="utf-8") if (SPECIMEN / "packet.md").exists() else ""
if "The command surface consolidated file listing into the list tool." not in packet:
    complaints.append(f"{SPECIMEN / 'packet.md'} no longer preserves the refused prose-shaped diff")

agreement_text = AGREEMENT.read_text(encoding="utf-8") if AGREEMENT.exists() else ""
if "docs/calibration/refused" in agreement_text or "blind-2026-09-first" in agreement_text:
    complaints.append(f"{AGREEMENT} names the refused specimen path")

trace = subprocess.run(
    ["bash", "-x", str(AGREEMENT)],
    check=False,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
)
combined = trace.stdout + trace.stderr
if trace.returncode != 0:
    complaints.append(f"{AGREEMENT} did not pass while proving specimen isolation")
if re.search(r"docs/calibration/refused|blind-2026-09-first", combined):
    complaints.append(f"{AGREEMENT} trace touched the refused specimen path")

if complaints:
    for complaint in complaints:
        print(complaint, file=sys.stderr)
    sys.exit(1)

print("refused blind audit specimen is kept and excluded from agreement evidence")
PY
