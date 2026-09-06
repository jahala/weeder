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
REPORT = pathlib.Path("docs/calibration-2026-09.md")
CASE_DIR = pathlib.Path("fixtures/adversarial/calibration-audit/blind-2026-09/cases")
SESSION_DIR = pathlib.Path("fixtures/adversarial/calibration-audit/blind-2026-09/sessions")
SPENT_SEEDS = {"calibration-audit-blind-2026-09-47"}
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


audit = read(AUDIT)
report = read(REPORT)
seed = field(audit, "Seed")

if seed in SPENT_SEEDS and "untrusted" not in first_prose_sentence(report).lower():
    complaints.append(f"{AUDIT} uses spent seed {seed} without an untrusted calibration verdict")

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
            for required in ("Catalogue:", "Weed findings:", "Diff cap bytes:", "Diff bytes:", "Diff capped:"):
                if required not in kept_text:
                    complaints.append(f"{kept_path} is missing {required}")
            if "Diff capped: no" in kept_text and "```diff" not in kept_text:
                complaints.append(f"{kept_path} is uncapped but carries no diff fence")
            if "Diff capped: yes" in kept_text and "exceeds the stated cap" not in kept_text:
                complaints.append(f"{kept_path} is capped without saying why the full diff is absent")
            if re.search(r"(?im)^Case: blocked:", kept_text):
                if not re.search(r"(?m)^\| [A-Z][0-9] \| error \| `[^`]+` \| [0-9]+ \| .+ \|$", kept_text):
                    complaints.append(f"{kept_path} has no printed weed finding row")
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
        answer = str(answer_event.get("answer", ""))
        if not answer.startswith(f"Answered packet SHA-256: {expected_hash}\n"):
            complaints.append(f"{session_path} answer does not open with {expected_hash}")
        for event in events:
            for forbidden_key in ("tool_calls", "tool_results", "repository_reads", "ledger_reads", "network_requests"):
                if event.get(forbidden_key):
                    complaints.append(f"{session_path} records forbidden {forbidden_key}")

if "untrusted" not in first_prose_sentence(report).lower():
    complaints.append(f"{REPORT}'s first prose sentence must say the classification is untrusted unless the fair blind audit stands")

if complaints:
    for complaint in complaints:
        print(complaint, file=sys.stderr)
    sys.exit(1)

print(f"fair blind case packets stand for seed {seed}")
PY
