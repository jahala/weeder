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

CASE_DIR = pathlib.Path("fixtures/adversarial/calibration-audit/blind-2026-09/cases")
SESSION_DIR = pathlib.Path("fixtures/adversarial/calibration-audit/blind-2026-09/sessions")
SIGHTED_DIR = pathlib.Path("fixtures/adversarial/calibration-audit/sighted-2026-09/sessions")
REPORT = pathlib.Path("docs/calibration-2026-09.md")
BLIND_AUDIT = pathlib.Path("docs/calibration-audit-blind-2026-09.md")
SIGHTED_AUDIT = pathlib.Path("docs/calibration-audit-2026-09.md")
complaints = []


def read(path):
    try:
        return pathlib.Path(path).read_text(encoding="utf-8")
    except FileNotFoundError:
        complaints.append(f"{path} is missing")
        return ""


def packet_hash(path):
    return hashlib.sha256(read(path).encode("utf-8")).hexdigest()


def audit_field(path, name):
    match = re.search(rf"(?im)^{re.escape(name)}:\s*(.+?)\s*$", read(path))
    if not match:
        complaints.append(f"{path} has no {name} line")
        return ""
    return match.group(1).strip().strip("`")


BLIND_MODEL = audit_field(BLIND_AUDIT, "Model")
SIGHTED_MODEL = audit_field(SIGHTED_AUDIT, "Model")


def named(path, input_event):
    """Every session record names the provider and the model that answered it.

    The seal is a population as much as a sample: an agreement number taken
    from one model says nothing about another, and this run is neither the
    provider nor the model the earlier seals were taken with.
    """
    provider = str(input_event.get("provider", "")).strip()
    model = str(input_event.get("model", "")).strip()
    expected = SIGHTED_MODEL if str(path).startswith(str(SIGHTED_DIR)) else BLIND_MODEL
    if not provider:
        complaints.append(f"{path} does not name the provider that answered it")
    if not model:
        complaints.append(f"{path} does not name the model that answered it")
    elif expected and model != expected:
        complaints.append(f"{path} names model {model}, and its audit file names {expected}")


def session_record(session_path, answer):
    """The provider's own event stream, beside the two-line summary.

    opencode prints one json event per line: a step start, the text parts the
    model wrote, and a step finish carrying the token counts. A part of any
    other kind is a tool, which is how a session reaches past its packet, and
    it is refused. The text parts joined are the answer the summary kept, so a
    summary cannot say something the session did not.
    """
    raw_path = session_path.with_name(
        session_path.name.replace(".events.jsonl", ".opencode.jsonl")
    )
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
            return
    kinds = [event.get("type") for event in raw_events]
    if "step_start" not in kinds or "step_finish" not in kinds:
        complaints.append(f"{raw_path} is not an opencode session record: no step_start and step_finish")
    sessions = {event.get("sessionID") for event in raw_events}
    if len(sessions) != 1 or not next(iter(sessions), None):
        complaints.append(f"{raw_path} does not carry one session id: {sorted(str(s) for s in sessions)}")
    part_kinds = sorted({(event.get("part") or {}).get("type") for event in raw_events})
    tools = [kind for kind in part_kinds if kind and "tool" in kind]
    if tools:
        complaints.append(f"{raw_path} shows tool parts, so the session reached past its packet: {tools}")
    if [kind for kind in part_kinds if kind not in ("step-start", "step-finish", "text", "reasoning")]:
        complaints.append(f"{raw_path} shows parts other than the auditor's answer: {part_kinds}")
    finishes = [event for event in raw_events if event.get("type") == "step_finish"]
    tokens = ((finishes[-1].get("part") or {}).get("tokens") or {}) if finishes else {}
    if not tokens.get("input"):
        complaints.append(f"{raw_path} records a step with no input tokens, so no packet was read")
    text = "".join(
        (event.get("part") or {}).get("text", "")
        for event in raw_events
        if event.get("type") == "text" and (event.get("part") or {}).get("type") == "text"
    ).strip()
    if not text:
        complaints.append(f"{raw_path} carries no answer text")
    elif text != answer:
        complaints.append(f"{raw_path} text differs from the recorded answer")


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
    allowed_input = {"type", "packet", "sha256", "provider", "model"}
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
    # This population was read by a different provider and a different model
    # from the ones the earlier seals were taken with, so the record says which
    # rather than leaving a reader to assume the run before it.
    named(session_path, input_event)
    answer = str(answer_event.get("answer", ""))
    if not answer.startswith(f"Answered packet SHA-256: {expected_hash}\n"):
        complaints.append(f"{session_path} answer does not open with the packet hash")
    # The two-line record is a summary; the session's own record is the raw
    # event stream the provider printed, kept beside it. A summary without a
    # stream behind it is a note, and a stream with a tool part in it was not
    # blind: a tool is how a session reaches past its packet.
    session_record(session_path, answer)

session_stems = {path.stem.removesuffix(".events") for path in SESSION_DIR.glob("*.events.jsonl")} if SESSION_DIR.is_dir() else set()
case_stems = {path.stem for path in case_paths}
if session_stems - case_stems:
    complaints.append(f"{SESSION_DIR} carries session records without case packets")

# The sighted half is anchored the same way, to the report rather than to a
# packet. Its answers open with the SHA-256 of the file they were given, and
# nothing used to read that back: the report's recall section was regenerated,
# every sighted answer went on naming a revision that no longer existed, and no
# check said so. A re-grade of a document is worth what the document was.
sighted = sorted(SIGHTED_DIR.glob("*.events.jsonl")) if SIGHTED_DIR.is_dir() else []
if not sighted:
    complaints.append(f"{SIGHTED_DIR} carries no session records")
else:
    report_hash = hashlib.sha256(read(REPORT).encode("utf-8")).hexdigest()
    for path in sighted:
        events = [json.loads(line) for line in read(path).splitlines() if line.strip()]
        answer = next((e.get("answer", "") for e in events if e.get("type") == "answer"), "")
        input_event = next((e for e in events if e.get("type") == "input"), {})
        named_hash = re.match(r"Answered report SHA-256: ([0-9a-f]{64})", answer)
        if named_hash is None:
            complaints.append(f"{path} answer does not open by naming the report it read")
        elif named_hash.group(1) != report_hash:
            complaints.append(
                f"{path} was answered against report {named_hash.group(1)[:12]} and {REPORT} is now "
                f"{report_hash[:12]}: the re-grade is of a document that has changed under it"
            )
        named(path, input_event)
        session_record(path, answer)

if complaints:
    for complaint in complaints:
        print(complaint, file=sys.stderr)
    sys.exit(1)

print(
    f"blind audit transcripts are packet-only: {len(case_paths)} sessions; "
    f"{len(sighted)} sighted sessions name the report as it stands"
)
PY
