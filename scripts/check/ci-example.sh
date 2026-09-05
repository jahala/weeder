#!/usr/bin/env bash
# Evidence for garden-fit: the GitHub Action example is weed's first
# distribution, so it is read as a contract rather than as documentation.
#
# What it has to do is one sentence: run weed against the pull request's base,
# write SARIF, and hand that file to GitHub's code scanning so every finding
# lands on the diff a reviewer is already looking at. Each half of that is
# checked here, and the whole file is put through actionlint, because an example
# that does not parse is worse than no example.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

example="examples/ci/github.yml"
[ -f "$example" ] || { echo "$example is missing: weed ships no way to run it in CI" >&2; exit 1; }

status=0

command -v actionlint >/dev/null 2>&1 || {
  echo "actionlint is not on PATH, so the example cannot be proved to parse. The check refuses to pass on unread yaml." >&2
  exit 3
}
command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the example cannot be read. The check refuses to pass on unread yaml." >&2
  exit 3
}

if ! actionlint "$example"; then
  echo "actionlint refused $example" >&2
  status=1
fi

python3 - "$example" <<'PY' || status=1
import re
import sys

import yaml

path = sys.argv[1]
with open(path, encoding="utf-8") as handle:
    workflow = yaml.safe_load(handle)

complaints = []


def steps(workflow):
    for name, job in (workflow.get("jobs") or {}).items():
        for step in job.get("steps") or []:
            yield name, step


run_steps = [(job, step) for job, step in steps(workflow) if step.get("run")]
uses_steps = [(job, step) for job, step in steps(workflow) if step.get("uses")]

# The command, read as an argument list rather than as a substring, so a flag
# that is merely mentioned in a comment cannot stand in for one that is passed.
checks = []
for job, step in run_steps:
    for line in step["run"].splitlines():
        line = line.strip()
        if re.match(r"^(\S*/)?weed\s+check\b", line):
            checks.append((job, line))

if not checks:
    complaints.append(f"{path} never runs `weed check`")

for job, line in checks:
    # The redirection is the point: the SARIF has to reach a file the upload
    # step can read.
    written = re.search(r">\s*(\S+\.sarif)\b", line)
    if written is None:
        complaints.append(f"{path}: `{line}` writes no .sarif file for the upload to read")
    # A `${{ ... }}` expression carries spaces and is still one argument, so it
    # is closed up before the line is split the way a shell would split it.
    closed = re.sub(r"\$\{\{\s*(.*?)\s*\}\}", lambda found: "${{" + found.group(1) + "}}", line)
    arguments = re.sub(r">\s*\S+", "", closed).split()
    for flag, value in (("--base", None), ("--strict", None), ("--format", "sarif")):
        if flag not in arguments:
            complaints.append(f"{path}: `{line}` does not pass {flag}")
        elif value is not None:
            at = arguments.index(flag)
            if at + 1 >= len(arguments) or arguments[at + 1] != value:
                complaints.append(f"{path}: `{line}` passes {flag} but not {value}")
    if "--base" in arguments:
        at = arguments.index("--base")
        base = arguments[at + 1] if at + 1 < len(arguments) else ""
        if "github.base_ref" not in base:
            complaints.append(
                f"{path}: `{line}` judges against `{base}` rather than the branch the pull "
                "request is opening onto"
            )

# The upload, and the file it is handed.
uploads = [
    (job, step)
    for job, step in uses_steps
    if step["uses"].split("@")[0].endswith("codeql-action/upload-sarif")
]
if not uploads:
    complaints.append(f"{path} never uploads its SARIF with github/codeql-action/upload-sarif")

written = {re.search(r">\s*(\S+\.sarif)\b", line).group(1) for _, line in checks if re.search(r">\s*(\S+\.sarif)\b", line)}
for job, step in uploads:
    named = (step.get("with") or {}).get("sarif_file")
    if named is None:
        complaints.append(f"{path}: the upload step names no sarif_file")
    elif written and named not in written:
        complaints.append(
            f"{path}: the upload reads {named}, and weed wrote {' '.join(sorted(written))}"
        )

# weed leaves with 2 when it blocks. If the upload is to happen at all, the run
# has to reach it, so the upload step says so outright.
for job, step in uploads:
    condition = str(step.get("if") or "")
    if "always()" not in condition:
        complaints.append(
            f"{path}: the upload step does not run on `if: always()`, so a blocked pull request "
            "uploads nothing and the reviewer sees no findings at all"
        )

# The permissions code scanning needs, declared where GitHub reads them.
def permissions_of(job_name):
    job = workflow["jobs"][job_name]
    return job.get("permissions") or workflow.get("permissions") or {}


for job, _ in uploads:
    granted = permissions_of(job)
    if granted.get("security-events") != "write":
        complaints.append(
            f"{path}: job '{job}' uploads SARIF without `security-events: write`, which is the "
            "one permission code scanning needs"
        )

# A shallow checkout has no base branch to diff against.
fetch_depth = None
for _, step in uses_steps:
    if step["uses"].split("@")[0] == "actions/checkout":
        fetch_depth = (step.get("with") or {}).get("fetch-depth")
if fetch_depth is None:
    complaints.append(
        f"{path}: actions/checkout does not set fetch-depth, and the default shallow clone has "
        "no base branch for --base to name"
    )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"{len(checks)} weed check run, {len(uploads)} SARIF upload")
PY

if [ "$status" -eq 0 ]; then
  echo "examples/ci/github.yml: parses, judges the pull request's base, and hands its SARIF to code scanning"
fi
exit "$status"
