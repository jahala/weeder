#!/usr/bin/env bash
# Evidence for windows: the suite runs on windows-latest on every push to master,
# never on a pull request, and the run on master says it passed.
#
# A cross-check from a developer machine cannot answer this. Every tree-sitter
# grammar weeder parses through is built by a C compiler, and building them for
# `x86_64-pc-windows-msvc` needs the MSVC toolchain, so `cargo check --target`
# stops long before it reaches weeder's own code. The compile can only be proven
# on a Windows runner, which is why this check reads a run rather than a tree.
#
# The owner ruled on 2026-11-06 that Windows never blocks a merge: the job lives
# in its own workflow, runs on push to master and on a schedule, and a failure
# opens or updates one issue labelled `windows`. So the yaml half holds the job
# to the suite ci.yml runs on ubuntu, refuses a pull-request trigger, and wants
# the report job; the GitHub half says the job on master's newest run went green.
#
# It refuses on the three ways this claim goes quietly false: a job that is not
# in the run at all, a job that was skipped, and a job that failed. None of them
# is allowed to read as a pass, because a leg nobody ran is exactly what v0.1.0
# published four platforms on.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

workflow=".github/workflows/windows.yml"
suite_workflow=".github/workflows/ci.yml"
branch="master"
runner="windows-latest"

[ -f "$workflow" ] || { echo "$workflow is missing, so nothing judges the windows platform" >&2; exit 1; }
[ -f "$suite_workflow" ] || { echo "$suite_workflow is missing, so there is no suite to hold windows to" >&2; exit 1; }

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the workflow cannot be read. The check refuses to pass on unread yaml." >&2
  exit 3
}
command -v gh >/dev/null 2>&1 || {
  echo "gh is not on PATH, so the run on $branch cannot be read. The check refuses to pass on a run nobody looked at." >&2
  exit 3
}
gh auth status >/dev/null 2>&1 || {
  echo "gh is not signed in, so the run on $branch cannot be read. The check refuses to pass on a run nobody looked at." >&2
  exit 3
}

# --- the yaml: a job on windows, after the fact, running ci.yml's suite --------
job="$(python3 - "$workflow" "$runner" "$suite_workflow" <<'PY'
import sys

import yaml

path, runner, suite_path = sys.argv[1], sys.argv[2], sys.argv[3]
with open(path, encoding="utf-8") as handle:
    workflow = yaml.safe_load(handle)
with open(suite_path, encoding="utf-8") as handle:
    suite_workflow = yaml.safe_load(handle)

# YAML reads a bare `on:` key as the boolean true.
triggers = workflow.get("on") or workflow.get(True) or {}
jobs = workflow.get("jobs") or {}
complaints = []


# A flag that changes how a run reports and not what it runs. `--no-fail-fast`
# keeps every suite running after one fails, so one Windows run says everything
# that is wrong there; the suite it runs is the same suite.
REPORTING_FLAGS = ("--no-fail-fast",)


def suites(job):
    """The `cargo test` lines a job runs, with reporting flags set aside."""
    found = []
    for step in job.get("steps") or []:
        line = (step.get("run") or "").strip()
        if "cargo test" not in line:
            continue
        words = [word for word in line.split() if word not in REPORTING_FLAGS]
        found.append(" ".join(words))
    return found


on_windows = {key: job for key, job in jobs.items() if job.get("runs-on") == runner}
if not on_windows:
    complaints.append(
        f"{path} has no job on {runner}: the release contract names five platforms and this is "
        f"the one nothing was ever compiled for before the tag"
    )

push = triggers.get("push") if isinstance(triggers, dict) else None
if "push" not in triggers or "master" not in ((push or {}).get("branches") or []):
    complaints.append(
        f"{path} does not run on a push to master, so nothing judges windows after a landing"
    )
if "pull_request" in triggers:
    complaints.append(
        f"{path} runs on pull_request, so windows holds merges up, which the owner ruled out "
        f"on 2026-11-06"
    )

report = [
    key
    for key, job in jobs.items()
    if key not in on_windows
    and "failure()" in str(job.get("if") or "")
    and set(on_windows) & set([job.get("needs")] if isinstance(job.get("needs"), str) else job.get("needs") or [])
    and any("gh issue" in (step.get("run") or "") for step in job.get("steps") or [])
]
if not report:
    complaints.append(
        f"{path} has no job that, when the windows job fails, opens or updates the windows "
        f"issue, so a red leg on master is a mark nobody reads"
    )

suite_jobs = suite_workflow.get("jobs") or {}
elsewhere = sorted({suite for job in suite_jobs.values() for suite in suites(job)})
for key, job in on_windows.items():
    if job.get("if"):
        complaints.append(
            f"{path}: job '{key}' carries `if: {job['if']}`, so the platform is judged on some "
            f"runs and not on others"
        )
    if job.get("continue-on-error"):
        complaints.append(f"{path}: job '{key}' continues on error, so nothing it runs can fail the build")
    ran = suites(job)
    if not ran:
        complaints.append(
            f"{path}: job '{key}' runs on {runner} and never runs the suite, so it proves the "
            f"compile and nothing about the behaviour"
        )
    for suite in ran:
        if suite not in elsewhere:
            complaints.append(
                f"{path}: job '{key}' runs `{suite}` and {suite_path} runs "
                f"{elsewhere or 'nothing'}: the platforms have to be held to one suite"
            )
    for step in job.get("steps") or []:
        if step.get("if"):
            complaints.append(
                f"{path}: a step of job '{key}' carries `if: {step['if']}`, so part of the suite "
                f"is judged conditionally"
            )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)

# The name GitHub gives the job in a run: its own `name` where it has one, and
# its key where it does not.
for key, job in on_windows.items():
    print(job.get("name") or key)
PY
)" || exit 1

echo "$workflow runs the suite on $runner as job '$job', on push to master and never on a pull request"

# --- the run: master's newest windows run, and what that job did in it --------
run="$(gh run list --workflow "$(basename "$workflow")" --branch "$branch" --limit 1 \
  --json databaseId,headSha,status,conclusion 2>/dev/null)" || {
  echo "gh could not list the runs of $workflow on $branch. The check refuses to pass on a run nobody looked at." >&2
  exit 3
}

python3 - "$branch" "$job" "$runner" "$run" <<'PY' || exit $?
import json
import subprocess
import sys

branch, wanted, runner, listed = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]

runs = json.loads(listed)
if not runs:
    print(
        f"there is no CI run on {branch} to read, so nothing says the {wanted} job passed there. "
        f"The check refuses to pass on a run that does not exist.",
        file=sys.stderr,
    )
    raise SystemExit(3)

run = runs[0]
identifier = run["databaseId"]
head = run["headSha"][:12]

viewed = subprocess.run(
    ["gh", "run", "view", str(identifier), "--json", "jobs"],
    capture_output=True,
    text=True,
)
if viewed.returncode != 0:
    print(
        f"gh could not read run {identifier}: {viewed.stderr.strip()}. The check refuses to pass "
        f"on a run nobody looked at.",
        file=sys.stderr,
    )
    raise SystemExit(3)

jobs = json.loads(viewed.stdout).get("jobs") or []
named = [job for job in jobs if job.get("name") == wanted]
if not named:
    print(
        f"run {identifier} on {branch} ({head}) has no {wanted} job: it ran "
        f"{[job.get('name') for job in jobs]}. A platform no run judges is the one the release "
        f"failed on.",
        file=sys.stderr,
    )
    raise SystemExit(1)

for job in named:
    status, conclusion = job.get("status"), job.get("conclusion")
    if status != "completed":
        print(
            f"the {wanted} job of run {identifier} on {branch} ({head}) is {status}, so nothing "
            f"has passed yet.",
            file=sys.stderr,
        )
        raise SystemExit(1)
    if conclusion != "success":
        print(
            f"the {wanted} job of run {identifier} on {branch} ({head}) is {conclusion}, not "
            f"success. A skipped or a red leg is what shipped four platforms out of five.",
            file=sys.stderr,
        )
        raise SystemExit(1)

print(
    f"the {wanted} job of run {identifier} on {branch} ({head}) passed: the suite is green on "
    f"{runner}"
)
PY

# --- the release gate: no tag ships a commit Windows has not passed -----------
# Pull requests no longer run Windows, so release.yml has to refuse a tagged
# commit whose windows.yml run is missing, unfinished or red. The yaml half says
# the gate runs in the job every build waits on; the run half drives the gate
# itself against real runs: master's newest green one passes, and the root
# commit, which no windows.yml run ever judged, is refused.
release=".github/workflows/release.yml"
gate="scripts/windows-green.sh"
[ -f "$gate" ] || { echo "$gate is missing, so nothing stops a tag on a commit Windows never passed" >&2; exit 1; }
python3 - "$release" "$gate" <<'PY' || exit 1
import sys

import yaml

path, gate = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as handle:
    jobs = (yaml.safe_load(handle) or {}).get("jobs") or {}
gating = [
    key
    for key, job in jobs.items()
    if any(gate in (step.get("run") or "") and "GITHUB_SHA" in (step.get("run") or "") for step in job.get("steps") or [])
]
if not gating:
    print(f"{path} never runs {gate} on the tagged commit, so a release can ship a commit Windows never passed", file=sys.stderr)
    raise SystemExit(1)
build = jobs.get("build") or {}
needs = build.get("needs")
needs = [needs] if isinstance(needs, str) else needs or []
if not set(gating) & set(needs):
    print(f"{path}: the build job does not wait on {gating[0]}, so it can publish before the Windows gate answers", file=sys.stderr)
    raise SystemExit(1)
PY

green_sha="$(gh run list --workflow "$(basename "$workflow")" --branch "$branch" --limit 1 --json headSha -q '.[0].headSha')"
if ! bash "$gate" "$green_sha" >/dev/null; then
  echo "$gate refused ${green_sha:0:12}, the commit of master's newest green windows run, so the gate refuses what it must let through" >&2
  exit 1
fi
root="$(git rev-list --max-parents=0 HEAD | head -1)"
if bash "$gate" "$root" >/dev/null 2>&1; then
  echo "$gate passed ${root:0:12}, the root commit, which no windows.yml run ever judged" >&2
  exit 1
fi
echo "$release runs $gate on the tagged commit before any build: it passes ${green_sha:0:12} and refuses ${root:0:12}"
