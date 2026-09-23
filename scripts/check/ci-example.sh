#!/usr/bin/env bash
# Evidence for garden-fit: the GitHub Action example is weeder's first
# distribution, so it is read as a contract rather than as documentation.
#
# What it has to do is one sentence: run weeder against the pull request's base,
# write SARIF, and hand that file to GitHub's code scanning so every finding
# lands on the diff a reviewer is already looking at. Each half of that is
# checked here, and the whole file is put through actionlint, because an example
# that does not parse is worse than no example.
#
# Then the example is run. Its install steps and its check step are read out of
# the yaml and handed to bash the way a GitHub runner hands them over, in a
# scratch clone of a repository whose pull request deletes a test, and in one
# whose pull request only adds a case. weeder has to arrive through the example's
# own install, and has to refuse the first and pass the second. The install
# downloads the latest release from GitHub, so this check needs the network: when
# github.com cannot be reached it says so and leaves with 3, because an install
# nobody ran is not an install that works.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

example="examples/ci/github.yml"
[ -f "$example" ] || { echo "$example is missing: weeder ships no way to run it in CI" >&2; exit 1; }

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
        if re.match(r"^(\S*/)?weeder\s+check\b", line):
            checks.append((job, line))

if not checks:
    complaints.append(f"{path} never runs `weeder check`")

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
            f"{path}: the upload reads {named}, and weeder wrote {' '.join(sorted(written))}"
        )

# weeder leaves with 2 when it blocks. If the upload is to happen at all, the run
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
print(f"{len(checks)} weeder check run, {len(uploads)} SARIF upload")
PY

[ "$status" -eq 0 ] || exit "$status"
echo "examples/ci/github.yml: parses, judges the pull request's base, and hands its SARIF to code scanning"

# --- the example, run ----------------------------------------------------------
#
# The one liberty taken: the example installs the x86_64 linux asset, and that
# binary runs on nothing else, so the step's WEEDER_TARGET is set to this
# machine's asset. On x86_64 linux it is the example's own value.
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) host=x86_64-unknown-linux-musl ;;
  Linux-aarch64 | Linux-arm64) host=aarch64-unknown-linux-musl ;;
  Darwin-x86_64) host=x86_64-apple-darwin ;;
  Darwin-arm64) host=aarch64-apple-darwin ;;
  *)
    echo "weeder publishes no release for $(uname -s) $(uname -m), so the example's install cannot be run here" >&2
    exit 3
    ;;
esac
for tool in git curl tar sha256sum; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool is not on PATH, and the example's steps need it, so they cannot be run here. The check refuses to pass on steps nobody ran." >&2
    exit 3
  }
done

scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT

ran=0
python3 - "$example" "$scratch" "$host" <<'PY' || ran=$?
import json
import os
import re
import shutil
import subprocess
import sys

import yaml

example, scratch, host = sys.argv[1:4]
workflow = yaml.safe_load(open(example, encoding="utf-8"))

# One job, its run steps in order. The steps up to the one that runs `weeder
# check` are the install; the check step is run once per pull request below.
jobs = workflow.get("jobs") or {}
if len(jobs) != 1:
    raise SystemExit(f"{example} has {len(jobs)} jobs, and the example is one job a reader copies")
(job_name, job), = jobs.items()
run_steps = [step for step in job.get("steps") or [] if step.get("run")]
checks = [
    at
    for at, step in enumerate(run_steps)
    if re.search(r"^\s*(\S*/)?weeder\s+check\b", step["run"], re.M)
]
if len(checks) != 1:
    raise SystemExit(f"{example} runs `weeder check` in {len(checks)} steps, and this runs exactly one")
install_steps, check_step = run_steps[: checks[0]], run_steps[checks[0]]
if not install_steps:
    raise SystemExit(f"{example} runs `weeder check` with no step before it that installs weeder")
if not any("WEEDER_TARGET" in (step.get("env") or {}) for step in install_steps):
    raise SystemExit(
        f"{example}'s install names no WEEDER_TARGET, so it cannot be pointed at this machine's asset"
    )

BASE = "main"
EXPRESSIONS = {"github.base_ref": BASE}


def resolve(text):
    """The `${{ }}` expressions the runner fills in, filled in the same way."""

    def one(found):
        expression = found.group(1).strip()
        if expression not in EXPRESSIONS:
            raise SystemExit(f"{example} uses `${{{{ {expression} }}}}`, which this run cannot fill in")
        return EXPRESSIONS[expression]

    return re.sub(r"\$\{\{(.*?)\}\}", one, str(text))


# git as a fresh runner has it: none of this machine's configuration, hooks or
# templates reach the scratch repositories or the weeder the example installs.
isolated = dict(os.environ)
isolated.update(
    {
        "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_AUTHOR_NAME": "a contributor",
        "GIT_AUTHOR_EMAIL": "contributor@example.invalid",
        "GIT_COMMITTER_NAME": "a contributor",
        "GIT_COMMITTER_EMAIL": "contributor@example.invalid",
    }
)


def git(cwd, *arguments):
    subprocess.run(["git", *arguments], cwd=cwd, env=isolated, check=True, capture_output=True)


def replace_tree(repository, source):
    for entry in os.listdir(repository):
        if entry != ".git":
            git(repository, "rm", "-r", "-q", "--", entry)
    shutil.copytree(source, repository, dirs_exist_ok=True)
    git(repository, "add", "-A")


def pull_request(name, fixture):
    """A clone of an origin whose main is the fixture's before and whose pull
    request is its after, checked out with full history as the example asks."""
    seed = os.path.join(scratch, f"{name}-seed")
    origin = os.path.join(scratch, f"{name}-origin.git")
    workspace = os.path.join(scratch, f"{name}-workspace")
    os.makedirs(seed)
    git(seed, "init", "-q", "-b", BASE)
    shutil.copytree(os.path.join(fixture, "before"), seed, dirs_exist_ok=True)
    git(seed, "add", "-A")
    git(seed, "commit", "-q", "-m", "the base branch")
    git(seed, "checkout", "-q", "-b", "pull")
    replace_tree(seed, os.path.join(fixture, "after"))
    git(seed, "commit", "-q", "-m", "the pull request")
    git(scratch, "clone", "-q", "--bare", seed, origin)
    git(scratch, "clone", "-q", origin, workspace)
    git(workspace, "checkout", "-q", "pull")
    return workspace


runner_temp = os.path.join(scratch, "runner-temp")
os.makedirs(runner_temp)
github_path = os.path.join(scratch, "github-path")
open(github_path, "w").close()


def step_environment(step, workspace):
    environment = dict(isolated)
    for scope in (workflow.get("env"), job.get("env"), step.get("env")):
        for key, value in (scope or {}).items():
            environment[key] = resolve(value)
    if "WEEDER_TARGET" in (step.get("env") or {}):
        environment["WEEDER_TARGET"] = host
    environment.update(
        {
            "CI": "true",
            "GITHUB_BASE_REF": BASE,
            "GITHUB_PATH": github_path,
            "GITHUB_WORKSPACE": workspace,
            "RUNNER_TEMP": runner_temp,
        }
    )
    # The runner puts every line a step appended to GITHUB_PATH in front of PATH
    # for the steps after it, the newest first.
    added = [line for line in open(github_path, encoding="utf-8").read().splitlines() if line]
    environment["PATH"] = os.pathsep.join([*reversed(added), environment["PATH"]])
    return environment


def run_step(step, workspace, label):
    script = os.path.join(scratch, f"{label}.sh")
    with open(script, "w", encoding="utf-8") as handle:
        handle.write(resolve(step["run"]))
    # A run step with no shell named is handed to `bash -e {0}` on linux and macos.
    shell = step.get("shell") or job.get("defaults", {}).get("run", {}).get("shell")
    if shell not in (None, "bash"):
        raise SystemExit(f"{example}: step '{step.get('name')}' runs under {shell}, and this runs bash")
    command = ["bash", "--noprofile", "--norc", "-eo", "pipefail", script] if shell else ["bash", "-e", script]
    return subprocess.run(
        command, cwd=workspace, env=step_environment(step, workspace), capture_output=True, text=True
    )


def reachable():
    probe = subprocess.run(
        ["curl", "-fsSI", "--max-time", "20", "-o", os.devnull, "https://github.com"],
        capture_output=True,
        text=True,
    )
    return probe.returncode == 0, probe.stderr.strip()


complaints = []
fixtures = os.path.join("fixtures", "adversarial", "T1", "rs")
blocked = pull_request("blocked", os.path.join(fixtures, "fire"))
clean = pull_request("clean", os.path.join(fixtures, "silent"))

for at, step in enumerate(install_steps):
    installed = run_step(step, blocked, f"install-{at}")
    if installed.returncode != 0:
        up, why = reachable()
        if not up:
            print(
                f"github.com cannot be reached ({why or 'no answer'}), so the example's install "
                "could not download weeder. Nothing was judged; run this again with the network.",
                file=sys.stderr,
            )
            raise SystemExit(3)
        raise SystemExit(
            f"{example}: step '{step.get('name')}' failed with {installed.returncode} while "
            f"github.com answers, so the example's install is broken:\n{installed.stdout}{installed.stderr}"
        )

# weeder has to be the one the install put on PATH, and nothing beside it.
found = subprocess.run(
    ["bash", "-c", "command -v weeder"],
    env=step_environment(check_step, blocked),
    capture_output=True,
    text=True,
).stdout.strip()
if not found or os.path.commonpath([os.path.realpath(found), os.path.realpath(scratch)]) != os.path.realpath(scratch):
    raise SystemExit(
        f"after the install, the check step would run {found or 'no weeder at all'}, which the "
        "example's install did not put there"
    )
beside = sorted(os.listdir(os.path.dirname(found)))
if beside != ["weeder"]:
    complaints.append(f"the install put {beside} where weeder runs from, and it extracts the one executable")
version = subprocess.run([found, "--version"], capture_output=True, text=True).stdout.strip()


def verdict(workspace, label):
    judged = run_step(check_step, workspace, label)
    written = os.path.join(workspace, "weeder.sarif")
    try:
        log = json.load(open(written, encoding="utf-8"))
    except (OSError, ValueError) as error:
        complaints.append(f"the {label} pull request left no SARIF to upload ({error}):\n{judged.stderr}")
        return judged.returncode, []
    if log.get("version") != "2.1.0":
        complaints.append(f"the {label} pull request's weeder.sarif is SARIF {log.get('version')}, not 2.1.0")
    results = [result for run in log.get("runs") or [] for result in run.get("results") or []]
    return judged.returncode, results


code, results = verdict(blocked, "blocked")
refusals = [r for r in results if r.get("ruleId") == "T1" and r.get("level") == "error"]
if code != 2 or not refusals:
    complaints.append(
        f"a pull request that deletes a test left the check step with {code} and "
        f"{len(refusals)} T1 refusals, and weeder refuses it with 2"
    )

code, results = verdict(clean, "clean")
errors = [r.get("ruleId") for r in results if r.get("level") == "error"]
if code != 0 or errors:
    complaints.append(
        f"a pull request that only adds a case left the check step with {code} and refusals "
        f"{errors}, and weeder passes it with 0"
    )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(
    f"{version} arrived through the example's install and its digest check; the check step "
    "refused a deleted test with 2 and passed an added case with 0"
)
PY

if [ "$ran" -ne 0 ]; then
  exit "$ran"
fi
echo "examples/ci/github.yml: installs weeder from the release, checks its digest, and gives weeder's verdict in a scratch repository"
