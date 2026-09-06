#!/usr/bin/env bash
# The sighted re-grade, run by code: one sampled case, one fresh auditor session
# whose input is the calibration report itself, one answer that names the file it
# was given.
#
#   bash scripts/audit/sighted-run.sh <seed> [concurrency]
#
# Sighted is the condition, not an accident of it: the report prints the
# builder's class and the builder's reasoning beside every blocked commit, so an
# auditor handed the report reads the answer before judging it. That is what this
# run records, and it is worth recording because the blind re-grade beside it is
# taken on packets with the class out of sight, and the two numbers are what the
# report's anchoring caveat is about.
#
# The sample is the seeded one `cargo xtask audit-packet` draws, so the sighted
# and the blind re-grade sample the same report by the same rule and differ only
# in what the auditor could see. Redoing the ledger moves that sample, which is
# why this is a script and not a file somebody typed once.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
seed="${1:?seed required}"
jobs="${2:-4}"
report="docs/calibration-2026-09.md"
base="fixtures/adversarial/calibration-audit/sighted-2026-09"
sessions="$base/sessions"

command -v codex >/dev/null 2>&1 || { echo "codex is not on PATH" >&2; exit 3; }
command -v trash >/dev/null 2>&1 || { echo "trash is not on PATH; nothing here deletes with rm" >&2; exit 3; }
[ -f "$report" ] || { echo "$report is missing: there is nothing for a sighted auditor to read" >&2; exit 3; }

# The sample, drawn by the same code the agreement check draws it with. The
# packets it writes are thrown away; only the case identities are wanted.
scratch="$(mktemp -d)"
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT
cargo xtask audit-packet --seed "$seed" --dir "$scratch/cases" >/dev/null

[ -d "$sessions" ] && trash "$sessions"
mkdir -p "$sessions"
count="$(ls "$scratch/cases"/*.md | wc -l | tr -d ' ')"
echo "$count sampled cases for seed $seed"

hash="$(shasum -a 256 "$report" | cut -d' ' -f1)"
export report sessions hash

# One session. Its whole input is the report, which carries the builder's class
# and reasoning beside each case, and the case it is asked about.
one() {
  case_path="$1"
  stem="$(basename "$case_path" .md)"
  case_id="$(grep -m1 '^Case: ' "$case_path" | sed 's/^Case: //')"
  raw="$sessions/$stem.codex.jsonl"
  events="$sessions/$stem.events.jsonl"
  empty="$(mktemp -d)"
  if [ "${case_id#blocked:}" != "$case_id" ]; then
    repo="$(echo "$case_id" | cut -d: -f2)"
    sha="$(echo "$case_id" | cut -d: -f3)"
    subject="the blocked commit $sha in $repo, which the report classifies in that repository's table"
    verdicts='true-positive, acceptable or false-positive: true-positive if the rule'"'"'s claim is true and the change really weakened something; acceptable if the claim is true and the change was fine anyway, so the block is friction the rule was designed to create; false-positive if the claim is not true of this change.'
  else
    rule="$(echo "$case_id" | cut -d: -f2)"
    sha="$(echo "$case_id" | cut -d: -f5)"
    path="$(echo "$case_id" | cut -d: -f6-)"
    subject="the recall case for rule $rule at \`$path\` in commit $sha, listed under 'Every miss'"
    verdicts='miss, caught or not-a-case: miss if the rule'"'"'s shape is genuinely present at the planted site and weed did not report it there; caught if weed did report it there; not-a-case if the shape is not genuinely present at that site.'
  fi
  prompt="You are re-grading one case for a calibration of a diff judge called weed. The calibration report on stdin is your only input; do not run commands, read files or use tools. Its SHA-256 is $hash. Re-grade $subject. Reply with exactly three lines and nothing else:
Answered report SHA-256: $hash
Verdict: <one of $verdicts>
Reasoning: <one sentence, under forty words>"
  if ! codex exec --json --ephemeral --skip-git-repo-check --ignore-user-config --ignore-rules -s read-only -C "$empty" "$prompt" < "$report" > "$raw" 2> "$sessions/$stem.stderr.txt"; then
    echo "codex exec failed on $stem" >&2
  fi
  trash "$empty" >/dev/null 2>&1 || true
  python3 - "$raw" "$events" "$case_id" "$hash" <<'PY'
import json, sys
raw, events, case_id, hash = sys.argv[1:5]
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
    out.write(json.dumps({"type": "input", "case": case_id, "sha256": hash}) + "\n")
    out.write(json.dumps({"type": "answer", "answer": answer}) + "\n")
print(("answered " if answer.startswith(f"Answered report SHA-256: {hash}") else "NO ANSWER ") + case_id)
PY
}
export -f one

ls "$scratch/cases"/*.md | xargs -P "$jobs" -I{} bash -c 'one "$@"' _ {}

# The audit file, from the answers and the report's own tables.
python3 - "$seed" "$base" "$scratch/cases" <<'PY'
import json, re, sys, pathlib
seed, base, cases = sys.argv[1:4]
base = pathlib.Path(base)
cases = pathlib.Path(cases)
report = open("docs/calibration-2026-09.md", encoding="utf-8").read()
classes = {}
for match in re.finditer(r"^\| `([0-9a-f]{7,})`[^|]*\| [^|]+ \| ([^|]+) \|", report, re.M):
    classes[match.group(1)[:10]] = match.group(2).strip().replace(" ", "-")
blocked, recall = [], []
for case_path in sorted(cases.glob("*.md")):
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
    "# calibration audit, 2026-09", "",
    "Provider: codex, OpenAI GPT-5 Codex (`codex exec`, one fresh session per case)", "",
    "Blind: no; the auditor could read the builder's classification and reasoning in docs/calibration-2026-09.md before judging.", "",
    f"Seed: {seed}", "",
    f"Sessions: {base}/sessions", "",
    f"The sample is the seeded one `cargo xtask audit-packet --seed {seed}` draws from the report, so this re-grade and the blind one beside it sample the same cases and differ only in what the auditor could see. `scripts/audit/sighted-run.sh` handed each case to a fresh `codex exec` session in an empty directory, with the user's configuration and rules ignored and the sandbox read-only, and the report itself as the session's only input. Each answer opens with the SHA-256 of the report it was given, and the raw event stream is kept beside it. The agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.", "",
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
pathlib.Path("docs/calibration-audit-2026-09.md").write_text("\n".join(out) + "\n", encoding="utf-8")
print(f"sighted audit written: blocked {b_agreed}/{len(blocked)}, recall {r_agreed}/{len(recall)}")
PY
