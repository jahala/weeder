#!/usr/bin/env bash
# Evidence for the release loop: the workflow a repository copies,
# `examples/ci/github.yml`, installs weeder from the release tarball, checks its
# SHA-256 before it runs it, and gets weeder's verdict.
#
# The example is one job that checks out the pull request and runs this
# repository's action, `action.yml`, by the tag of this version. So both are
# read: the example for what it asks of GitHub, the permissions, the trigger,
# the checkout, the tag, and the action for the order it does things in. The
# example goes through actionlint, because an example that does not parse is
# worse than no example.
#
# Then the action is run, its own steps handed to bash the way a runner hands
# them over. First its install alone, with nothing named, which downloads the
# release garden.json names for this machine from GitHub and checks its digest,
# so this check needs the network: when github.com cannot be reached it says so
# and leaves with 3, because an install nobody ran is not an install that works.
# Then the whole action, in a scratch clone of a repository whose pull request
# deletes a test and of one whose pull request only adds a case, installing the
# weeder this tree builds through the action's `archive` input, since a tree
# ahead of its release judges with flags the release does not have yet. weeder
# has to refuse the first and pass the second, with the comment's body in the
# job summary both times. The comment itself needs a pull request on GitHub,
# and scripts/check/action-comment.sh reads that one.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

example="examples/ci/github.yml"
[ -f "$example" ] || { echo "$example is missing: weeder ships no way to run it in CI" >&2; exit 1; }
[ -f action.yml ] || { echo "action.yml is missing: the example runs an action that is not there" >&2; exit 1; }

for tool in actionlint python3; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool is not on PATH, so the example cannot be proved to parse and be read. The check refuses to pass on unread yaml." >&2
    exit 3
  }
done

status=0
if ! actionlint "$example"; then
  echo "actionlint refused $example" >&2
  status=1
fi

python3 - "$example" <<'PY' || status=1
import json
import sys

import yaml

path = sys.argv[1]
workflow = yaml.safe_load(open(path, encoding="utf-8"))
action = yaml.safe_load(open("action.yml", encoding="utf-8"))
version = json.load(open("garden.json", encoding="utf-8"))["version"]
complaints = []

# PyYAML reads the bare key `on` as the boolean true.
on = workflow.get("on", workflow.get(True)) or {}
events = [on] if isinstance(on, str) else list(on)
if "pull_request" not in events:
    complaints.append(f"{path} does not run on pull_request")
if "pull_request_target" in events:
    complaints.append(f"{path} runs on pull_request_target, which hands a fork's code a token that can write")

jobs = workflow.get("jobs") or {}
if len(jobs) != 1:
    complaints.append(f"{path} has {len(jobs)} jobs, and the example is one job a reader copies")
for name, job in jobs.items():
    granted = job.get("permissions") or {}
    if granted != {"contents": "read", "pull-requests": "write"}:
        complaints.append(f"{path}: job '{name}' asks for {granted}, and the comment needs contents: read and pull-requests: write and nothing else")
    for key, value in (workflow.get("permissions") or {}).items():
        if (key, value) != ("contents", "read"):
            complaints.append(f"{path} grants {key}: {value} above the job")
    steps = job.get("steps") or []
    uses = [step.get("uses", "") for step in steps]
    checkout = [step for step in steps if step.get("uses", "").split("@")[0] == "actions/checkout"]
    if not checkout or (checkout[0].get("with") or {}).get("fetch-depth") != 0:
        complaints.append(f"{path}: actions/checkout does not set fetch-depth: 0, and a shallow clone has no base branch to judge against")
    tag = f"jahala/weeder@v{version}"
    if tag not in uses:
        complaints.append(f"{path} does not run the action by the tag of this version, {tag}; it uses {uses}")
    if any(step.get("run") for step in steps):
        complaints.append(f"{path} runs commands of its own, and the action is the one home of installing and running weeder")

steps = (action.get("runs") or {}).get("steps") or []
ids = [step.get("id") for step in steps]
if ids != ["install", "judge", "summary", "comment", "verdict"]:
    complaints.append(f"action.yml runs {ids}, and it installs, judges, writes the summary and the comment, and decides last")
install = next((step.get("run", "") for step in steps if step.get("id") == "install"), "")
digest, extract = install.find("SHA-256"), install.find("tar xzf")
if digest < 0 or extract < 0 or digest > extract:
    complaints.append("action.yml's install does not check the archive's SHA-256 before it opens it")
if "garden.json" not in install:
    complaints.append("action.yml's install does not read the release out of the action's own garden.json")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"{path} runs jahala/weeder@v{version} with contents: read and pull-requests: write")
PY

[ "$status" -eq 0 ] || exit "$status"
echo "examples/ci/github.yml: parses, asks for what the comment needs, and runs the action by this version's tag"

# --- the action, run -----------------------------------------------------------

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) os=Linux arch=X64 ;;
  Linux-aarch64 | Linux-arm64) os=Linux arch=ARM64 ;;
  Darwin-x86_64) os=macOS arch=X64 ;;
  Darwin-arm64) os=macOS arch=ARM64 ;;
  *)
    echo "weeder publishes no release for $(uname -s) $(uname -m), so the action's install cannot be run here" >&2
    exit 3
    ;;
esac
for tool in git curl tar jq node cargo; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool is not on PATH, and the action's steps or this tree's build need it, so they cannot be run here. The check refuses to pass on steps nobody ran." >&2
    exit 3
  }
done

cargo build --quiet --bin weeder

scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT

bash scripts/package-release.sh this-machine target/debug "$scratch/release" >/dev/null

ran=0
python3 - "$scratch" "$os" "$arch" <<'PY' || ran=$?
import json
import os
import re
import shutil
import subprocess
import sys

import yaml

scratch, runner_os, runner_arch = sys.argv[1:4]
root = os.getcwd()
action = yaml.safe_load(open("action.yml", encoding="utf-8"))
steps = action["runs"]["steps"]
version = json.load(open("garden.json", encoding="utf-8"))["version"]
archive = "file://" + os.path.join(scratch, "release", "weeder-this-machine.tar.gz")
BASE = "main"

# git as a fresh runner has it: none of this machine's configuration, hooks or
# templates reach the scratch repositories or the weeder the action installs.
isolated = {
    "PATH": os.environ["PATH"],
    "HOME": os.path.join(scratch, "home"),
    "GH_CONFIG_DIR": os.path.join(scratch, "gh"),
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_AUTHOR_NAME": "a contributor",
    "GIT_AUTHOR_EMAIL": "contributor@example.invalid",
    "GIT_COMMITTER_NAME": "a contributor",
    "GIT_COMMITTER_EMAIL": "contributor@example.invalid",
}
for directory in ("home", "gh"):
    os.makedirs(os.path.join(scratch, directory), exist_ok=True)


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


def run(label, workspace, chosen, only=None):
    """The action's steps in order, each only if the one before it passed, the
    way a runner runs a composite action. The token is empty, as on a job with
    no right to write, so the comment step goes nowhere."""
    temp = os.path.join(scratch, f"{label}-temp")
    os.makedirs(temp)
    files = {name: os.path.join(scratch, f"{label}-{name}") for name in ("path", "summary")}
    for name in files.values():
        open(name, "w").close()
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=workspace, capture_output=True, text=True).stdout.strip()
    outputs, codes, said = {}, {}, {}
    for at, step in enumerate(steps):
        if only and step["id"] not in only:
            continue
        known = {
            "inputs.token": "",
            "inputs.archive": chosen,
            "github.action_path": root,
            "github.base_ref": BASE,
            "github.event_name": "pull_request",
            "github.server_url": "https://github.com",
            "github.repository": "someone/somewhere",
            "github.event.pull_request.number": "1",
            "github.event.pull_request.head.sha": head,
            "runner.os": runner_os,
            "runner.arch": runner_arch,
            "runner.temp": temp,
            **outputs,
        }

        def fill(found):
            expression = found.group(1).strip()
            if expression not in known:
                raise SystemExit(f"action.yml uses `${{{{ {expression} }}}}`, which this run cannot fill in")
            return known[expression]

        output = os.path.join(scratch, f"{label}-output-{at}")
        open(output, "w").close()
        added = [line for line in open(files["path"], encoding="utf-8").read().splitlines() if line]
        environment = dict(isolated)
        environment.update(
            {
                "PATH": os.pathsep.join([*reversed(added), isolated["PATH"]]),
                "GITHUB_OUTPUT": output,
                "GITHUB_PATH": files["path"],
                "GITHUB_STEP_SUMMARY": files["summary"],
                "RUNNER_TEMP": temp,
            }
        )
        for key, value in (step.get("env") or {}).items():
            environment[key] = re.sub(r"\$\{\{(.*?)\}\}", fill, str(value))
        script = os.path.join(scratch, f"{label}-{at}.sh")
        open(script, "w", encoding="utf-8").write(step["run"])
        done = subprocess.run(
            ["bash", "--noprofile", "--norc", "-eo", "pipefail", script],
            cwd=workspace, env=environment, capture_output=True, text=True,
        )
        for line in open(output, encoding="utf-8").read().splitlines():
            key, _, value = line.partition("=")
            outputs[f"steps.{step['id']}.outputs.{key}"] = value
        codes[step["id"]] = done.returncode
        said[step["id"]] = done.stdout + done.stderr
        if done.returncode != 0:
            break
    return codes, said, open(files["summary"], encoding="utf-8").read()


def reachable():
    probe = subprocess.run(
        ["curl", "-fsSI", "--max-time", "20", "-o", os.devnull, "https://github.com"],
        capture_output=True, text=True,
    )
    return probe.returncode == 0, probe.stderr.strip()


complaints = []
fixtures = os.path.join("fixtures", "adversarial", "T1", "rs")
blocked = pull_request("blocked", os.path.join(fixtures, "fire"))
clean = pull_request("clean", os.path.join(fixtures, "silent"))

# The release, as anyone who copies the example installs it.
codes, said, _ = run("release", blocked, "", only={"install"})
if codes.get("install") != 0:
    up, why = reachable()
    if not up:
        print(
            f"github.com cannot be reached ({why or 'no answer'}), so the action's install could not "
            "download weeder. Nothing was judged; run this again with the network.",
            file=sys.stderr,
        )
        raise SystemExit(3)
    raise SystemExit(f"the action's install of the release failed while github.com answers:\n{said.get('install')}")
if f"weeder {version}" not in said["install"]:
    complaints.append(f"the action installed something other than weeder {version}:\n{said['install']}")

# This tree's weeder, through the whole action.
for label, workspace, verdict in (("blocked", blocked, 2), ("clean", clean, 0)):
    codes, said, summary = run(label, workspace, archive)
    if list(codes) != ["install", "judge", "summary", "comment", "verdict"]:
        complaints.append(f"the {label} pull request ran {list(codes)}:\n{json.dumps(said, indent=1)}")
        continue
    if codes["verdict"] != verdict:
        complaints.append(f"the {label} pull request left the job with {codes['verdict']}, and weeder decides it with {verdict}")
    if codes["comment"] != 0:
        complaints.append(f"the comment step changed the {label} job's result: {said['comment']}")
    if not summary.startswith("<!-- weeder"):
        complaints.append(f"the {label} pull request's job summary does not carry the comment:\n{summary}")
    if verdict == 2 and "`T1`" not in summary:
        complaints.append(f"the blocked pull request's summary names no T1:\n{summary}")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(
    f"weeder {version} arrived through the action's install and its digest check; the action, "
    "with this tree's weeder, refused a deleted test with 2 and passed an added case with 0, "
    "the summary written both times"
)
PY

if [ "$ran" -ne 0 ]; then
  exit "$ran"
fi
echo "examples/ci/github.yml: the action installs weeder from the release, checks its digest, and gives weeder's verdict in a scratch repository"
