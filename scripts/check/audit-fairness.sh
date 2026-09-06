#!/usr/bin/env bash
# Evidence for calibration-audit.tend2.html: the blind packet is regenerated
# byte-for-byte and carries fair context from weed, not judgement ledger prose.
set -euo pipefail

python3 - <<'PY'
import hashlib
import pathlib
import re
import subprocess
import sys

AUDIT = pathlib.Path("docs/calibration-audit-blind-2026-09.md")
REPORT = pathlib.Path("docs/calibration-2026-09.md")
PACKET = pathlib.Path("docs/calibration-audit-blind-2026-09.packet.md")
complaints = []


def read(path):
    try:
        return pathlib.Path(path).read_text(encoding="utf-8")
    except FileNotFoundError:
        complaints.append(f"{path} is missing")
        return ""


def field(text, name):
    match = re.search(rf"(?im)^{re.escape(name)}:\s*(.+?)\s*$", text)
    if not match:
        complaints.append(f"{AUDIT} has no {name} line")
        return ""
    return match.group(1).strip().strip("`")


def sections(packet, prefix):
    return re.findall(rf"(?ms)^### {prefix}:[^\n]+\n\n(.*?)(?=^### |\Z)", packet)


audit = read(AUDIT)
packet = read(PACKET)
report = read(REPORT)
seed = field(audit, "Seed")
recorded_hash = field(audit, "Packet SHA-256")

if packet and seed:
    regenerated = subprocess.run(
        ["cargo", "xtask", "audit-packet", "--seed", seed],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if regenerated.returncode != 0:
        complaints.append("cargo xtask audit-packet could not regenerate the blind packet: " + regenerated.stderr.strip())
    elif regenerated.stdout != packet:
        complaints.append(f"{PACKET} differs from a freshly regenerated fair packet")

if packet and recorded_hash:
    actual_hash = hashlib.sha256(packet.encode("utf-8")).hexdigest()
    if actual_hash != recorded_hash:
        complaints.append(f"{PACKET} hash is {actual_hash}, not {recorded_hash}")
    if recorded_hash not in audit:
        complaints.append(f"{AUDIT} does not record the packet hash")

blocked = sections(packet, "blocked")
recall = sections(packet, "recall")
if len(blocked) != 20:
    complaints.append(f"{PACKET} carries {len(blocked)} blocked sections, not 20")
if len(recall) != 20:
    complaints.append(f"{PACKET} carries {len(recall)} recall sections, not 20")

for index, section in enumerate(blocked, 1):
    for required in ("Catalogue:", "Weed findings:", "```diff"):
        if required not in section:
            complaints.append(f"blocked packet section {index} is missing {required}")
    if not re.search(r"(?m)^\| [A-Z][0-9] \| error \| `[^`]+` \| [0-9]+ \| .+ \|$", section):
        complaints.append(f"blocked packet section {index} has no weed finding row with rule, path, line and message")
    if not re.search(r"(?m)^[A-Z][0-9]\s+\S+\s+check\s+", section):
        complaints.append(f"blocked packet section {index} has no `weed rules` catalogue line")

for index, section in enumerate(recall, 1):
    for required in ("Catalogue:", "Weed findings:", "Planted site:", "Question:", "```diff"):
        if required not in section:
            complaints.append(f"recall packet section {index} is missing {required}")
    if "genuinely present at that planted site" not in section:
        complaints.append(f"recall packet section {index} does not ask the shape/site question")
    if not re.search(r"(?m)^[A-Z][0-9]\s+\S+\s+check\s+", section):
        complaints.append(f"recall packet section {index} has no `weed rules` catalogue line")
    if not re.search(r"(?m)^\| ([A-Z][0-9]|none) \|", section):
        complaints.append(f"recall packet section {index} has no weed finding table row")

for forbidden in (
    "Classification | Why",
    "true positive |",
    "false positive |",
    "acceptable |",
    "docs/calibration/judgements.toml",
    "docs/calibration/refused/",
):
    if forbidden.lower() in packet.lower():
        complaints.append(f"{PACKET} carries forbidden ledger or refused-specimen material: {forbidden}")

first_sentence = re.split(r"(?<=[.!?])\s+", report.strip(), maxsplit=1)[0]
if "untrusted" not in first_sentence.lower():
    complaints.append(f"{REPORT}'s first sentence must say the classification is untrusted while the fair blind audit is below the bar or absent")

if complaints:
    for complaint in complaints:
        print(complaint, file=sys.stderr)
    sys.exit(1)

print(f"fair blind packet stands: {recorded_hash}")
PY
