#!/usr/bin/env bash
# Evidence for pr-comment: on a real pull request in this repository, the
# action keeps one comment and a job that cannot write still passes.
#
# A test cannot answer this without standing in for GitHub, so this reads
# GitHub. `.github/workflows/comment.yml` runs the action on every pull request
# in two legs: `write`, which may write on the pull request, and `read`, which
# has `pull-requests: read`, the right a pull request from a fork gets. The
# claim holds when, for one pull request:
#
#   - the Comment workflow ran for at least two pushes, two different head
#     commits, and its newest run judged the commit the pull request is at;
#   - exactly one comment opens with the action's marker, it was written before
#     the newest run and edited during it, and its text is the body that run's
#     write leg judged, as its job log carries it;
#   - the newest run's read leg could not write, said so as a warning, put the
#     same body in its job summary, and passed;
#   - code scanning holds an analysis in the `weeder` category on the pull
#     request's merge ref from the newest run, the write leg's upload.
#
# The pull request is the one named as the first argument, or the one whose head
# is the branch checked out here. A run, a leg or a log that is not there is a
# failure rather than a pass, because a claim nobody could read is not a claim
# that holds.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

for tool in gh python3; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool is not on PATH, so the pull request cannot be read from GitHub. The check refuses to pass on a pull request nobody looked at." >&2
    exit 3
  }
done

repository="$(gh repo view --json nameWithOwner --jq .nameWithOwner)" || {
  echo "gh could not name this repository on GitHub, so there is no pull request to read." >&2
  exit 3
}
if [ "$#" -ge 1 ]; then
  number="$1"
else
  number="$(gh pr view --json number --jq .number)" || {
    echo "no pull request has this branch as its head. name one: bash $0 <number>" >&2
    exit 3
  }
fi

scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT

gh api "repos/$repository/pulls/$number" > "$scratch/pull.json"
branch="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["head"]["ref"])' "$scratch/pull.json")"
gh api --paginate "repos/$repository/issues/$number/comments" --jq '.[]' > "$scratch/comments.jsonl"
gh api --paginate "repos/$repository/actions/workflows/comment.yml/runs?event=pull_request&branch=$branch&per_page=100" \
  --jq '.workflow_runs[]' > "$scratch/runs.jsonl"

newest="$(python3 - "$scratch" "$number" <<'PY'
import json
import os
import sys

scratch, number = sys.argv[1], int(sys.argv[2])
runs = [json.loads(line) for line in open(os.path.join(scratch, "runs.jsonl")) if line.strip()]
runs = [
    run for run in runs
    if run.get("status") == "completed"
    and any(pull.get("number") == number for pull in run.get("pull_requests") or [])
]
if not runs:
    raise SystemExit(f"the Comment workflow has no completed run for pull request {number}")
runs.sort(key=lambda run: run["run_started_at"])
print(runs[-1]["id"])
PY
)"
gh api "repos/$repository/actions/runs/$newest/jobs" --jq '.jobs[]' > "$scratch/jobs.jsonl"
while IFS= read -r job; do
  gh api "repos/$repository/actions/jobs/$job/logs" > "$scratch/log-$job.txt"
done < <(python3 -c 'import json,sys; [print(json.loads(l)["id"]) for l in open(sys.argv[1]) if l.strip()]' "$scratch/jobs.jsonl")

# What code scanning holds from weeder for this pull request: the write leg
# uploads as `auto` does on a public repository, so the newest run leaves an
# analysis in the `weeder` category on the pull request's merge ref.
gh api "repos/$repository/code-scanning/analyses?ref=refs/pull/$number/merge&per_page=100" \
  --jq '.[]' > "$scratch/analyses.jsonl"

python3 - "$scratch" "$number" "$newest" <<'PY'
import json
import os
import re
import sys

scratch, number, newest = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
complaints = []


def lines(name):
    return [json.loads(line) for line in open(os.path.join(scratch, name)) if line.strip()]


source = open("src/core/comment.rs", encoding="utf-8").read()
marker = re.search(r'pub const MARKER: &str = "(.*?)";', source).group(1)
pull = json.load(open(os.path.join(scratch, "pull.json")))
runs = [
    run for run in lines("runs.jsonl")
    if run.get("status") == "completed"
    and any(p.get("number") == number for p in run.get("pull_requests") or [])
]
runs.sort(key=lambda run: run["run_started_at"])
run = next(run for run in runs if run["id"] == newest)

pushes = {run["head_sha"] for run in runs}
if len(pushes) < 2:
    complaints.append(f"the Comment workflow ran for {len(pushes)} head commit of pull request {number}, and the claim needs two pushes")
if run["head_sha"] != pull["head"]["sha"]:
    complaints.append(f"the newest run judged {run['head_sha'][:12]}, and the pull request is at {pull['head']['sha'][:12]}")

jobs = {job["name"]: job for job in lines("jobs.jsonl")}

uploaded = [
    analysis for analysis in lines("analyses.jsonl")
    if analysis.get("category") == "weeder" and analysis.get("created_at", "") >= run["run_started_at"]
]
if not uploaded:
    complaints.append("code scanning holds no `weeder` analysis on this pull request from its newest run, so the write leg's upload did not land")


def log(job):
    text = open(os.path.join(scratch, f"log-{job['id']}.txt"), encoding="utf-8", errors="replace").read()
    # Every line of a job log opens with the time it was written.
    return [re.sub(r"^\S+Z ", "", line, count=1) for line in text.splitlines()]


def body(job):
    """The comment's body as the summary step wrote it to the job's log."""
    said = log(job)
    try:
        start = next(i for i, line in enumerate(said) if line.startswith("##[group]the comment, as the job summary carries it"))
        end = next(i for i in range(start, len(said)) if said[i].startswith("##[endgroup]"))
    except StopIteration:
        return None
    return "\n".join(said[start + 1:end]).strip()


legs = {}
for name in ("write", "read"):
    job = jobs.get(name)
    if job is None:
        complaints.append(f"the newest run has no `{name}` leg; it has {sorted(jobs)}")
        continue
    legs[name] = job
    if job.get("conclusion") != "success":
        complaints.append(f"the `{name}` leg of the newest run concluded {job.get('conclusion')}")

carrying = [comment for comment in lines("comments.jsonl") if comment["body"].startswith(marker)]
if len(carrying) != 1:
    complaints.append(f"{len(carrying)} comments on pull request {number} open with weeder's marker, and the action keeps one")

if "write" in legs and len(carrying) == 1:
    comment, job = carrying[0], legs["write"]
    if not comment["created_at"] < run["run_started_at"]:
        complaints.append("the comment was written by the newest run rather than by an earlier one, so nothing shows it was edited")
    if not job["started_at"] <= comment["updated_at"] <= job["completed_at"]:
        complaints.append(
            f"the comment was last changed at {comment['updated_at']}, outside the newest write leg "
            f"({job['started_at']} to {job['completed_at']})"
        )
    written = body(job)
    if written is None:
        complaints.append("the newest write leg's log carries no body, so what it judged cannot be read")
    elif written != comment["body"].strip():
        complaints.append("the comment's text is not the body the newest write leg judged")
    if not any("weeder edited its comment" in line for line in log(job)):
        complaints.append("the newest write leg did not say it edited the comment")

if "read" in legs:
    job = legs["read"]
    said = log(job)
    if not any(line.startswith("##[warning]") and "could not" in line for line in said):
        complaints.append("the read leg did not say it could not write the comment")
    read_body = body(job)
    if read_body is None:
        complaints.append("the read leg's log carries no body, so its summary cannot be shown to hold it")
    elif "write" in legs and read_body != body(legs["write"]):
        complaints.append("the read leg's summary is not the body the write leg judged")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(
    f"pull request {number}: {len(pushes)} pushes, one comment edited by run {newest}, and its read leg "
    "put the same body in its summary and passed"
)
PY
