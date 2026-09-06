#!/usr/bin/env bash
# Evidence for calibration-audit.tend2.html: the blind audit transcript names
# the packet as the session's only input and carries no signs of tools, source
# reads, ledger reads, refused specimen reads or network access.
set -euo pipefail

python3 - <<'PY'
import hashlib
import pathlib
import re
import sys

AUDIT = pathlib.Path("docs/calibration-audit-blind-2026-09.md")
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


audit = read(AUDIT)
packet_path = field(audit, "Packet")
response_path = field(audit, "Response")
transcript_path = field(audit, "Transcript")
recorded_hash = field(audit, "Packet SHA-256")

packet = read(packet_path) if packet_path else ""
response = read(response_path) if response_path else ""
transcript = read(transcript_path) if transcript_path else ""

if packet and recorded_hash:
    actual_hash = hashlib.sha256(packet.encode("utf-8")).hexdigest()
    if actual_hash != recorded_hash:
        complaints.append(f"{packet_path} hash is {actual_hash}, not {recorded_hash}")

if transcript:
    lowered = transcript.lower()
    if "blind: yes" not in lowered:
        complaints.append(f"{transcript_path} does not declare blindness")
    for required in (packet_path, response_path, recorded_hash):
        if required and required not in transcript:
            complaints.append(f"{transcript_path} does not name {required}")
    forbidden = [
        r"\bfunctions\.",
        r"\bmcp__",
        r"\bweb\.run\b",
        r"\btool_code\b",
        r"\btool_result\b",
        r"\bexec_command\b",
        r"\bbash\s+-lc\b",
        r"\bgit\s+(show|diff|log|cat-file|checkout|rev-parse|fetch)\b",
        r"\brg\s+",
        r"\bgrep\s+",
        r"\bcat\s+docs/",
        r"https?://",
        r"docs/calibration-2026-09\.md",
        r"docs/calibration/judgements\.toml",
        r"docs/calibration/refused/",
    ]
    for pattern in forbidden:
        if re.search(pattern, transcript, re.I):
            complaints.append(f"{transcript_path} shows forbidden session material matching {pattern}")
    if packet and packet[:200] in transcript:
        complaints.append(f"{transcript_path} embeds packet bytes instead of naming the packet file as input")
    if response and response[:80] not in transcript and "response is kept verbatim" not in lowered:
        complaints.append(f"{transcript_path} does not account for the auditor response")

if complaints:
    for complaint in complaints:
        print(complaint, file=sys.stderr)
    sys.exit(1)

print(f"blind audit transcript is packet-only: {transcript_path}")
PY
