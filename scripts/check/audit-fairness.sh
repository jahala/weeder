#!/usr/bin/env bash
# Evidence for calibration-audit.tend2.html: the fair blind packet is one
# regenerated, hash-linked packet per sampled case, not a packet edited by hand.
set -euo pipefail

python3 - <<'PY'
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile

AUDIT = pathlib.Path("docs/calibration-audit-blind-2026-09.md")
SIGHTED_AUDIT = pathlib.Path("docs/calibration-audit-2026-09.md")
REPORT = pathlib.Path("docs/calibration-2026-09.md")
CASE_DIR = pathlib.Path("fixtures/adversarial/calibration-audit/blind-2026-09/cases")
SESSION_DIR = pathlib.Path("fixtures/adversarial/calibration-audit/blind-2026-09/sessions")
# Every seed that has been answered. A re-grade re-runs on a seed nobody has
# seen, so an answer cannot be a memory of the run before it.
SPENT_SEEDS = {
    "calibration-audit-blind-2026-09-47",
    "calibration-audit-blind-2026-09-fair-1",
    "calibration-audit-blind-2026-09-redo-2",
    "calibration-audit-blind-2026-09-t2-1",
}
# The findings table is the judged content and nothing else: a rule id, a
# level, a path and a line. A fifth cell would be the finding's sentence, and a
# sentence in the sealed bytes is what made a rename cost forty sessions.
FINDINGS_HEADER = "| Rule | Level | Path | Line |"
FINDING_ROW = re.compile(
    r"^\| (?:none|[A-Z][0-9]) \| (?:none|error|warning|note) \| (?:`[^`]*`)? \| [0-9]+ \|$"
)
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


def first_prose_sentence(text):
    prose = "\n".join(
        line for line in text.splitlines() if line.strip() and not line.lstrip().startswith("#")
    ).strip()
    return re.split(r"(?<=[.!?])\s+", prose, maxsplit=1)[0] if prose else ""


def sha256_text(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def judged_content_only(path, text):
    """The findings a packet carries, read as a table of judged content.

    A finding is a rule id, a level, a path and a line. Its sentence says the
    same thing for a person, and it is left out of the packet, because the seal
    the auditors' answers hang on is the sha256 of these bytes: a message
    reworded, or a token respelled inside one, would move every hash and buy
    forty sessions again. What the auditor judges the change against is the
    rule's own catalogue line, which is printed above the table.
    """
    complaints = []
    body = re.split(r"(?m)^```", text)[::2]
    for section in body:
        match = re.search(r"(?m)^Weeder findings:\n\n(.*?)(?:\n\n|\Z)", section, re.S)
        if match is None:
            continue
        rows = match.group(1).splitlines()
        if not rows or rows[0].strip() != FINDINGS_HEADER:
            complaints.append(f"{path} findings table is not {FINDINGS_HEADER}: {rows[:1]}")
            continue
        for row in rows[2:]:
            if not FINDING_ROW.match(row.strip()):
                complaints.append(f"{path} findings row carries more than the judged content: {row}")
    if re.search(r"(?m)^\| Rule \| Level \| Path \| Line \| Message \|", text):
        complaints.append(f"{path} prints a finding message column")
    return complaints


audit = read(AUDIT)
report = read(REPORT)
seed = field(audit, "Seed")
model = field(audit, "Model")
# The agreement is a number about a population: this one was read by a provider
# and a model neither earlier seal used, so both audit files say which.
for path in (AUDIT, SIGHTED_AUDIT):
    text = read(path)
    for name in ("Provider", "Model"):
        if not re.search(rf"(?im)^{name}:\s*\S", text):
            complaints.append(f"{path} does not name the {name.lower()} that produced it")

if seed in SPENT_SEEDS:
    complaints.append(f"{AUDIT} re-uses seed {seed}, which has been answered already")

with tempfile.TemporaryDirectory() as tmp:
    regenerated_dir = pathlib.Path(tmp) / "cases"
    regenerated = subprocess.run(
        ["cargo", "xtask", "audit-packet", "--seed", seed, "--dir", str(regenerated_dir)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if regenerated.returncode != 0:
        complaints.append("cargo xtask audit-packet --dir could not regenerate case packets: " + regenerated.stderr.strip())
    else:
        kept = sorted(CASE_DIR.glob("*.md"))
        fresh = sorted(regenerated_dir.glob("*.md"))
        if len(kept) != 40:
            complaints.append(f"{CASE_DIR} carries {len(kept)} case packets, not 40")
        if [path.name for path in kept] != [path.name for path in fresh]:
            complaints.append(f"{CASE_DIR} case names differ from regenerated packets")
        for kept_path, fresh_path in zip(kept, fresh):
            kept_text = read(kept_path)
            fresh_text = read(fresh_path)
            if kept_text != fresh_text:
                complaints.append(f"{kept_path} differs from regenerated {fresh_path.name}")
                continue
            for required in ("Catalogue:", "Weeder findings:", "Diff cap bytes:", "Diff bytes:", "Diff capped:"):
                if required not in kept_text:
                    complaints.append(f"{kept_path} is missing {required}")
            if "Diff capped: no" in kept_text and "```diff" not in kept_text:
                complaints.append(f"{kept_path} is uncapped but carries no diff fence")
            if "Diff capped: yes" in kept_text and "exceeds the stated cap" not in kept_text:
                complaints.append(f"{kept_path} is capped without saying why the full diff is absent")
            if re.search(r"(?im)^Case: blocked:", kept_text):
                if not re.search(r"(?m)^\| [A-Z][0-9] \| error \| `[^`]+` \| [0-9]+ \|$", kept_text):
                    complaints.append(f"{kept_path} has no printed weeder finding row")
            complaints.extend(judged_content_only(kept_path, kept_text))
            if re.search(r"(?im)^Case: recall:", kept_text):
                for required in ("Planted site:", "Question:", "genuinely present at that planted site"):
                    if required not in kept_text:
                        complaints.append(f"{kept_path} is missing recall context {required}")
            for forbidden in (
                "Classification | Why",
                "docs/calibration/judgements.toml",
                "docs/calibration/refused/",
            ):
                if forbidden.lower() in kept_text.lower():
                    complaints.append(f"{kept_path} carries forbidden ledger or refused-specimen material: {forbidden}")

if CASE_DIR.exists():
    for case_path in sorted(CASE_DIR.glob("*.md")):
        case_text = read(case_path)
        expected_hash = sha256_text(case_text)
        session_path = SESSION_DIR / f"{case_path.stem}.events.jsonl"
        session_text = read(session_path)
        if not session_text:
            continue
        try:
            events = [json.loads(line) for line in session_text.splitlines() if line.strip()]
        except json.JSONDecodeError as error:
            complaints.append(f"{session_path} is not valid jsonl: {error}")
            continue
        if len(events) != 2:
            complaints.append(f"{session_path} has {len(events)} events, not one input and one answer")
            continue
        input_event, answer_event = events
        if input_event.get("type") != "input":
            complaints.append(f"{session_path} first event is not input")
        if answer_event.get("type") != "answer":
            complaints.append(f"{session_path} second event is not answer")
        if input_event.get("packet") != str(case_path):
            complaints.append(f"{session_path} input names {input_event.get('packet')}, not {case_path}")
        if input_event.get("sha256") != expected_hash:
            complaints.append(f"{session_path} input hash is not {expected_hash}")
        if not str(input_event.get("provider", "")).strip():
            complaints.append(f"{session_path} does not name the provider that answered it")
        if str(input_event.get("model", "")).strip() != model:
            complaints.append(
                f"{session_path} names model {input_event.get('model')}, and {AUDIT} names {model}"
            )
        answer = str(answer_event.get("answer", ""))
        if not answer.startswith(f"Answered packet SHA-256: {expected_hash}\n"):
            complaints.append(f"{session_path} answer does not open with {expected_hash}")
        for event in events:
            for forbidden_key in ("tool_calls", "tool_results", "repository_reads", "ledger_reads", "network_requests"):
                if event.get(forbidden_key):
                    complaints.append(f"{session_path} records forbidden {forbidden_key}")

# The audit stands when every sample in its own agreement table is at the bar
# on at least twenty cases; the agreement script recomputes those rows, and
# this one reads them. While it does not stand, the report has to say so.
rows = re.findall(r"^\| ([a-z ]+) \| (\d+) \| (\d+) \| [0-9.]+% \|$", audit, re.M)
stands = bool(rows) and all(int(n) >= 20 and int(a) * 100 >= 90 * int(n) for _, n, a in rows)
if not stands and "untrusted" not in first_prose_sentence(report).lower():
    complaints.append(f"{REPORT}'s first prose sentence must say the classification is untrusted while the fair blind audit is under the bar")

# The packet's shape is held above on the packets in the tree; the property
# behind it is held at the function that writes the table, where a message
# rewritten between two runs has to leave the bytes where they were.
unit = subprocess.run(
    ["cargo", "test", "--quiet", "--package", "xtask", "--bin", "xtask", "audit_packet::"],
    check=False,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
)
if unit.returncode != 0:
    complaints.append(
        "the packet's seal is not held at the generator: "
        + (unit.stdout + unit.stderr).strip().splitlines()[-1]
    )

if complaints:
    for complaint in complaints:
        print(complaint, file=sys.stderr)
    sys.exit(1)

print(f"fair blind case packets stand for seed {seed}, answered on {model}")
PY
