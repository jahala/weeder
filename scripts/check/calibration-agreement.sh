#!/usr/bin/env bash
# Evidence for calibration-audit.tend2.html: the audit file must carry a
# reproducible blind re-grade sample and the agreement it earns.
set -euo pipefail

report="docs/calibration-2026-09.md"
audit="docs/calibration-audit-2026-09.md"

python3 - "$report" "$audit" <<'PY'
import hashlib
import os
import re
import sys

report_path, audit_path = sys.argv[1:3]
complaints = []

CLASSIFICATIONS = {
    "true positive": "true-positive",
    "true-positive": "true-positive",
    "acceptable": "acceptable",
    "false positive": "false-positive",
    "false-positive": "false-positive",
}


def cell_text(cell):
    return re.sub(r"`([^`]*)`", r"\1", cell.strip()).strip()


def verdict(cell):
    text = cell_text(cell).lower()
    return CLASSIFICATIONS.get(text, text)


def table_cells(line):
    line = line.strip()
    if not line.startswith("|"):
        return None
    cells = [cell.strip() for cell in line.strip("|").split("|")]
    if all(re.fullmatch(r":?-{3,}:?", cell.replace(" ", "")) for cell in cells):
        return None
    return cells


def seeded_sample(items, seed, label, count=20):
    return sorted(
        items,
        key=lambda item: hashlib.sha256(
            f"{seed}\0{label}\0{item['key']}".encode("utf-8")
        ).hexdigest(),
    )[:count]


def read(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return handle.read()
    except FileNotFoundError:
        complaints.append(f"{path} is missing")
        return ""


report = read(report_path)
audit = read(audit_path)

provider_match = re.search(r"(?im)^provider:\s*(.+?)\s*$", audit)
if not provider_match or not provider_match.group(1).strip():
    complaints.append(f"{audit_path} does not name the provider that produced it")

seed_match = re.search(r"(?im)^seed:\s*([A-Za-z0-9._:-]+)\s*$", audit)
seed = seed_match.group(1) if seed_match else ""
if not seed:
    complaints.append(f"{audit_path} does not record a seeded shuffle")

blocked = []
repo = None
in_recall = False
for line in report.splitlines():
    if line.strip() == "<!-- recall:begin -->":
        in_recall = True
    if in_recall:
        continue
    heading = re.match(r"^##\s+([A-Za-z0-9_-]+),\s+\d+ commits judged,", line)
    if heading:
        repo = heading.group(1)
        continue
    cells = table_cells(line)
    if not repo or not cells or len(cells) < 4:
        continue
    commit_match = re.match(r"`?([0-9a-f]{10,40})`?\s+(.*)", cells[0])
    if not commit_match:
        continue
    original = verdict(cells[2])
    if original not in CLASSIFICATIONS.values():
        continue
    sha = commit_match.group(1)
    blocked.append(
        {
            "key": f"{repo}:{sha}",
            "repo": repo,
            "sha": sha,
            "original": original,
        }
    )

recall = []
recall_section = re.search(r"### Every miss\n\n(.*?)(?:\n### |\n<!-- recall:end -->)", report, re.S)
if recall_section:
    for line in recall_section.group(1).splitlines():
        match = re.match(
            r"^- ([A-Z][0-9]) · ([a-z]+) · ([A-Za-z0-9_-]+) `([0-9a-f]{9,40})` · `([^`]+)`, (.+)$",
            line.strip(),
        )
        if not match:
            continue
        rule, lang, repo_name, sha, path, reason = match.groups()
        recall.append(
            {
                "key": f"{rule}:{lang}:{repo_name}:{sha}:{path}",
                "rule": rule,
                "lang": lang,
                "repo": repo_name,
                "sha": sha,
                "path": path,
                "original": "miss",
                "reason": reason,
            }
        )
else:
    complaints.append(f"{report_path} carries no recall miss list to sample")

if len(blocked) < 20:
    complaints.append(f"{report_path} names only {len(blocked)} blocked commits; twenty are required")
if len(recall) < 20:
    complaints.append(f"{report_path} names only {len(recall)} recall cases; twenty are required")

blocked_sample = seeded_sample(blocked, seed, "blocked") if seed else []
recall_sample = seeded_sample(recall, seed, "recall") if seed else []

blocked_audit = {}
recall_audit = {}
agreement_rows = {}
section = None
for line in audit.splitlines():
    lowered = line.strip().lower()
    if lowered.startswith("## "):
        if "blocked commit" in lowered:
            section = "blocked"
        elif "recall case" in lowered:
            section = "recall"
        elif "agreement" in lowered:
            section = "agreement"
        else:
            section = None
        continue
    cells = table_cells(line)
    if not cells:
        continue
    header = [cell.lower() for cell in cells]
    if "repo" in header or "sample" in header or "rule" in header:
        continue
    if section == "blocked" and len(cells) >= 4:
        repo_name = cell_text(cells[0])
        sha = cell_text(cells[1])
        blocked_audit[f"{repo_name}:{sha}"] = {
            "verdict": verdict(cells[2]),
            "reason": cell_text(cells[3]),
        }
    elif section == "recall" and len(cells) >= 7:
        rule = cell_text(cells[0])
        lang = cell_text(cells[1])
        repo_name = cell_text(cells[2])
        sha = cell_text(cells[3])
        path = cell_text(cells[4])
        recall_audit[f"{rule}:{lang}:{repo_name}:{sha}:{path}"] = {
            "verdict": verdict(cells[5]).lower(),
            "reason": cell_text(cells[6]),
        }
    elif section == "agreement" and len(cells) >= 3:
        try:
            agreement_rows[cell_text(cells[0]).lower()] = (int(cells[1]), int(cells[2]))
        except ValueError:
            pass


def has_reason(row):
    reason = row.get("reason", "").strip()
    return len(reason) >= 20 and re.search(r"[A-Za-z]", reason)


def check_sample(name, sample, audit_rows):
    agreed = 0
    for item in sample:
        row = audit_rows.get(item["key"])
        if row is None:
            complaints.append(f"{audit_path} omits sampled {name} case {item['key']}")
            continue
        if not has_reason(row):
            complaints.append(f"{audit_path} gives sampled {name} case {item['key']} no reasoning")
        if row["verdict"] == item["original"]:
            agreed += 1
    return len(sample), agreed


blocked_count, blocked_agreed = check_sample("blocked", blocked_sample, blocked_audit)
recall_count, recall_agreed = check_sample("recall", recall_sample, recall_audit)

expected_rows = {
    "blocked commits": (blocked_count, blocked_agreed),
    "recall cases": (recall_count, recall_agreed),
}
for sample_name, expected in expected_rows.items():
    actual = agreement_rows.get(sample_name)
    if actual != expected:
        complaints.append(
            f"{audit_path} agreement row for {sample_name} is {actual}, expected {expected}"
        )
    count, agreed = expected
    if count < 20:
        complaints.append(f"{sample_name} sample has {count} cases; twenty are required")
    elif agreed * 100 < 90 * count:
        complaints.append(
            f"{sample_name} agreement is {agreed}/{count}, below the 90 percent bar"
        )

first_sentence = re.split(r"(?<=[.!?])\s+", report.strip(), maxsplit=1)[0]
if any(agreed * 100 < 90 * count for count, agreed in expected_rows.values()):
    if "untrusted" not in first_sentence.lower():
        complaints.append(
            f"{report_path}'s first sentence does not say the classification is untrusted"
        )

extra_blocked = set(blocked_audit) - {item["key"] for item in blocked_sample}
extra_recall = set(recall_audit) - {item["key"] for item in recall_sample}
if extra_blocked:
    complaints.append(f"{audit_path} carries blocked rows outside the seeded sample")
if extra_recall:
    complaints.append(f"{audit_path} carries recall rows outside the seeded sample")

if complaints:
    for complaint in complaints:
        print(complaint, file=sys.stderr)
    sys.exit(1)

print(
    f"calibration audit agreement stands: blocked commits {blocked_agreed}/{blocked_count}, "
    f"recall cases {recall_agreed}/{recall_count}"
)
PY
