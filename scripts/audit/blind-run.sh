#!/usr/bin/env bash
# The blind re-grade, run by code: one case packet, one fresh auditor session
# whose only input is that packet, one answer that names the packet's hash.
#
#   bash scripts/audit/blind-run.sh <seed> [concurrency]
#
# `cargo xtask audit-packet --dir` writes the case packets from the pinned
# corpus. Each is handed to `codex exec` on stdin, in an empty directory, with
# the user's config and rules ignored and the sandbox read-only, so the session
# has nothing to read but the packet. The raw event stream codex prints is kept
# beside the case as `<case>.codex.jsonl`, the session's own record; the two-line
# `<case>.events.jsonl` beside it is the input hash and the answer, which the
# evidence scripts read. The audit file is then assembled from the answers and
# the report's own tables, and the agreement script recomputes every number.
#
# Nothing here reads the ledger's reasoning, and the only thing a person types
# is the seed.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
seed="${1:?seed required}"
jobs="${2:-4}"
base="docs/calibration-audit-blind-2026-09"
cases="$base/cases"
sessions="$base/sessions"
only="${WEED_AUDIT_ONLY:-}"

command -v codex >/dev/null 2>&1 || { echo "codex is not on PATH" >&2; exit 3; }
command -v trash >/dev/null 2>&1 || { echo "trash is not on PATH; nothing here deletes with rm" >&2; exit 3; }

[ -d "$cases" ] && trash "$cases"
[ -d "$sessions" ] && trash "$sessions"
mkdir -p "$cases" "$sessions"
cargo xtask audit-packet --seed "$seed" --dir "$cases" >/dev/null
count="$(ls "$cases"/*.md | wc -l | tr -d ' ')"
echo "$count case packets for seed $seed"

# One session. The prompt carries the packet's hash so the answer can name it;
# the hash is machine-derived and says nothing about the case.
one() {
  case_path="$1"
  stem="$(basename "$case_path" .md)"
  hash="$(shasum -a 256 "$case_path" | cut -d' ' -f1)"
  raw="$sessions/$stem.codex.jsonl"
  events="$sessions/$stem.events.jsonl"
  empty="$(mktemp -d)"
  if grep -q '^Case: blocked:' "$case_path"; then
    verdicts='true-positive, acceptable or false-positive: true-positive if the rule'"'"'s claim is true and the change really weakened something; acceptable if the claim is true and the change was fine anyway, so the block is friction the rule was designed to create; false-positive if the claim is not true of this change. Judge whether the rule'"'"'s claim, as printed in the weed finding, is true of the change, not whether the change is bad.'
  else
    verdicts='miss, caught or not-a-case: miss if the rule'"'"'s shape is genuinely present at the planted site and the weed findings do not report it there; caught if the weed findings do report it there; not-a-case if the shape is not genuinely present at that site.'
  fi
  prompt="You are re-grading one case for a calibration of a diff judge called weed. The packet on stdin is your only input; do not run commands, read files or use tools. Its SHA-256 is $hash. Reply with exactly three lines and nothing else:
Answered packet SHA-256: $hash
Verdict: <one of $verdicts>
Reasoning: <one sentence, under forty words>"
  if ! codex exec --json --ephemeral --skip-git-repo-check --ignore-user-config --ignore-rules -s read-only -C "$empty" "$prompt" < "$case_path" > "$raw" 2> "$sessions/$stem.stderr.txt"; then
    echo "codex exec failed on $stem" >&2
  fi
  trash "$empty" >/dev/null 2>&1 || true
  python3 - "$raw" "$events" "$case_path" "$hash" <<'PY'
import json, sys
raw, events, case_path, hash = sys.argv[1:5]
answer = ""
for line in open(raw, encoding="utf-8"):
    line = line.strip()
    if not line:
        continue
    try:
        event = json.loads(line)
    except json.JSONDecodeError:
        continue
    item = event.get("item") or {}
    if event.get("type") == "item.completed" and item.get("type") == "agent_message":
        answer = item.get("text", "")
with open(events, "w", encoding="utf-8") as out:
    out.write(json.dumps({"type": "input", "packet": case_path, "sha256": hash}) + "\n")
    out.write(json.dumps({"type": "answer", "answer": answer}) + "\n")
print(("answered " if answer.startswith(f"Answered packet SHA-256: {hash}") else "NO ANSWER ") + case_path.split("/")[-1])
PY
}
export -f one
export sessions

if [ -n "$only" ]; then
  one "$cases/$only.md"
  exit 0
fi
ls "$cases"/*.md | xargs -P "$jobs" -I{} bash -c 'one "$@"' _ {}

# The audit file, from the answers and the report's own tables.
python3 - "$seed" "$base" <<'PY'
import json, re, sys, pathlib
seed, base = sys.argv[1:3]
base = pathlib.Path(base)
report = open("docs/calibration-2026-09.md", encoding="utf-8").read()
classes = {}
for match in re.finditer(r"^\| `([0-9a-f]{7,})`[^|]*\| [^|]+ \| ([^|]+) \|", report, re.M):
    classes[match.group(1)[:10]] = match.group(2).strip().replace(" ", "-")
blocked, recall = [], []
for case_path in sorted((base / "cases").glob("*.md")):
    text = case_path.read_text(encoding="utf-8")
    case = re.search(r"(?m)^Case: (.+)$", text).group(1).strip()
    events = [json.loads(l) for l in (base / "sessions" / f"{case_path.stem}.events.jsonl").read_text(encoding="utf-8").splitlines() if l.strip()]
    answer = events[1]["answer"] if len(events) == 2 else ""
    verdict = re.search(r"(?m)^Verdict:\s*(\S+)", answer)
    reason = re.search(r"(?m)^Reasoning:\s*(.+)$", answer)
    verdict = verdict.group(1).strip().strip(".").lower() if verdict else "no-answer"
    reason = reason.group(1).strip() if reason else "the session returned no answer in the shape asked for"
    reason = reason.replace("|", "/")
    if case.startswith("blocked:"):
        _, repo, sha = case.split(":", 2)
        blocked.append((repo, sha, verdict, reason, classes.get(sha[:10], "?")))
    else:
        _, rule, lang, repo, sha, path = case.split(":", 5)
        recall.append((rule, lang, repo, sha, path, verdict, reason))
b_agreed = sum(1 for row in blocked if row[2] == row[4])
r_agreed = sum(1 for row in recall if row[5] == "miss")
def pct(a, n): return f"{(a * 100.0 / n) if n else 0.0:.1f}%"
out = [
    "# calibration audit blind, 2026-09", "",
    "Provider: codex, OpenAI GPT-5 Codex (`codex exec`, one fresh session per case)", "",
    "Blind: yes", "",
    f"Seed: {seed}", "",
    f"Cases: {base}/cases", "",
    f"Sessions: {base}/sessions", "",
    f"Every case packet was written by `cargo xtask audit-packet --seed {seed} --dir {base}/cases` from the pinned corpus, and `scripts/audit/blind-run.sh` handed each one to a fresh `codex exec` session in an empty directory, with the user's configuration and rules ignored and the sandbox read-only, as the session's only input. The raw event stream of each session is kept beside its case, and each answer opens with the SHA-256 of the packet it was given. Nothing from the ledger reached a session; the agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.", "",
    "## Agreement", "",
    "| Sample | Re-graded | Agreed | Agreement |", "|---|---:|---:|---:|",
    f"| blocked commits | {len(blocked)} | {b_agreed} | {pct(b_agreed, len(blocked))} |",
    f"| recall cases | {len(recall)} | {r_agreed} | {pct(r_agreed, len(recall))} |", "",
    "## Blocked Commit Sample", "",
    "| Repo | Commit | Auditor verdict | Reasoning |", "|---|---|---|---|",
]
out += [f"| {repo} | `{sha[:10]}` | {verdict} | {reason} |" for repo, sha, verdict, reason, _ in blocked]
out += ["", "## Recall Case Sample", "", "| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |", "|---|---|---|---|---|---|---|"]
out += [f"| {rule} | {lang} | {repo} | `{sha}` | `{path}` | {verdict} | {reason} |" for rule, lang, repo, sha, path, verdict, reason in recall]
(base.parent / "calibration-audit-blind-2026-09.md").write_text("\n".join(out) + "\n", encoding="utf-8")
print(f"blind audit written: blocked {b_agreed}/{len(blocked)}, recall {r_agreed}/{len(recall)}")
PY
