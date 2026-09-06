#!/usr/bin/env bash
# Evidence for calibration-audit.tend2.html: every sampled case has its own
# packet-only session record, with one input event and one answer event.
set -euo pipefail

python3 - <<'PY'
import hashlib
import json
import pathlib
import re
import sys

CASE_DIR = pathlib.Path("docs/calibration-audit-blind-2026-09/cases")
SESSION_DIR = pathlib.Path("docs/calibration-audit-blind-2026-09/sessions")
complaints = []


def read(path):
    try:
        return pathlib.Path(path).read_text(encoding="utf-8")
    except FileNotFoundError:
        complaints.append(f"{path} is missing")
        return ""


def packet_hash(path):
    return hashlib.sha256(read(path).encode("utf-8")).hexdigest()


for directory in (CASE_DIR, SESSION_DIR):
    if not directory.is_dir():
        complaints.append(f"{directory} is missing")

case_paths = sorted(CASE_DIR.glob("*.md")) if CASE_DIR.is_dir() else []
if len(case_paths) != 40:
    complaints.append(f"{CASE_DIR} carries {len(case_paths)} case packets, not 40")

for case_path in case_paths:
    expected_hash = packet_hash(case_path)
    session_path = SESSION_DIR / f"{case_path.stem}.events.jsonl"
    text = read(session_path)
    if not text:
        continue
    lowered = text.lower()
    forbidden_patterns = [
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
    for pattern in forbidden_patterns:
        if re.search(pattern, lowered, re.I):
            complaints.append(f"{session_path} shows forbidden session material matching {pattern}")
    try:
        events = [json.loads(line) for line in text.splitlines() if line.strip()]
    except json.JSONDecodeError as error:
        complaints.append(f"{session_path} is not valid jsonl: {error}")
        continue
    if len(events) != 2:
        complaints.append(f"{session_path} has {len(events)} events, not one input and one answer")
        continue
    input_event, answer_event = events
    allowed_input = {"type", "packet", "sha256"}
    allowed_answer = {"type", "answer"}
    if set(input_event) - allowed_input:
        complaints.append(f"{session_path} input event has extra keys {sorted(set(input_event) - allowed_input)}")
    if set(answer_event) - allowed_answer:
        complaints.append(f"{session_path} answer event has extra keys {sorted(set(answer_event) - allowed_answer)}")
    if input_event.get("type") != "input":
        complaints.append(f"{session_path} first event is not input")
    if answer_event.get("type") != "answer":
        complaints.append(f"{session_path} second event is not answer")
    if input_event.get("packet") != str(case_path):
        complaints.append(f"{session_path} input names {input_event.get('packet')}, not {case_path}")
    if input_event.get("sha256") != expected_hash:
        complaints.append(f"{session_path} input hash is {input_event.get('sha256')}, not {expected_hash}")
    answer = str(answer_event.get("answer", ""))
    if not answer.startswith(f"Answered packet SHA-256: {expected_hash}\n"):
        complaints.append(f"{session_path} answer does not open with the packet hash")
    # The two-line record is a summary; the session's own record is the raw
    # event stream codex printed, kept beside it. A summary without a stream
    # behind it is a note, and a stream with a command in it was not blind.
    raw_path = session_path.with_name(session_path.name.replace(".events.jsonl", ".codex.jsonl"))
    raw_text = read(raw_path)
    raw_events = []
    for raw_line in raw_text.splitlines():
        raw_line = raw_line.strip()
        if not raw_line:
            continue
        try:
            raw_events.append(json.loads(raw_line))
        except json.JSONDecodeError:
            complaints.append(f"{raw_path} carries a line that is not json")
            break
    kinds = [event.get("type") for event in raw_events]
    if "thread.started" not in kinds or "turn.completed" not in kinds:
        complaints.append(f"{raw_path} is not a codex session record: no thread.started and turn.completed")
    completed = [event for event in raw_events if event.get("type") == "turn.completed"]
    if completed and not (completed[-1].get("usage") or {}).get("input_tokens"):
        complaints.append(f"{raw_path} records a turn with no input tokens, so no packet was read")
    items = [event.get("item") or {} for event in raw_events if event.get("type") == "item.completed"]
    item_kinds = sorted({item.get("type") for item in items})
    messages = [item for item in items if item.get("type") == "agent_message"]
    if [kind for kind in item_kinds if kind not in ("agent_message", "reasoning")]:
        complaints.append(f"{raw_path} shows items other than the auditor's message: {item_kinds}")
    if len(messages) != 1:
        complaints.append(f"{raw_path} carries {len(messages)} agent messages, not one")
    elif messages[0].get("text", "") != answer:
        complaints.append(f"{raw_path} message differs from the recorded answer")

session_stems = {path.stem.removesuffix(".events") for path in SESSION_DIR.glob("*.events.jsonl")} if SESSION_DIR.is_dir() else set()
case_stems = {path.stem for path in case_paths}
if session_stems - case_stems:
    complaints.append(f"{SESSION_DIR} carries session records without case packets")

if complaints:
    for complaint in complaints:
        print(complaint, file=sys.stderr)
    sys.exit(1)

print(f"blind audit transcripts are packet-only: {len(case_paths)} sessions")
PY
