#!/usr/bin/env bash
# Evidence for garden-fit: the release pipeline, in tilth's shape.
#
# Nothing here is run against GitHub, that is the owner's action, so the
# workflows are proved the only ways they can be proved on this machine: they
# parse and pass actionlint, and the claims that matter are read out of the yaml
# and checked against the repository they release. A workflow that names a
# target the manifest does not, or a version the crate does not, is a release
# that goes wrong at the one moment nobody is watching.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

status=0

for file in .github/workflows/ci.yml .github/workflows/release.yml npm/package.json npm/install.js npm/run.js; do
  if [ ! -f "$file" ]; then
    echo "$file is missing" >&2
    status=1
  fi
done
[ "$status" -eq 0 ] || exit "$status"

command -v actionlint >/dev/null 2>&1 || {
  echo "actionlint is not on PATH, so the workflows cannot be proved to parse. The check refuses to pass on unread yaml." >&2
  exit 3
}
command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the workflows cannot be read. The check refuses to pass on unread yaml." >&2
  exit 3
}

if ! actionlint .github/workflows/ci.yml .github/workflows/release.yml; then
  echo "actionlint refused the workflows" >&2
  status=1
fi

python3 - <<'PY' || status=1
import json
import re
import sys

import yaml


def load(path):
    with open(path, encoding="utf-8") as handle:
        return yaml.safe_load(handle)


def steps(workflow):
    """Every step of every job, with the job it belongs to."""
    for name, job in (workflow.get("jobs") or {}).items():
        for step in job.get("steps") or []:
            yield name, job, step


def runs(step):
    return step.get("run") or ""


complaints = []

# The version three files have to agree on. Cargo.toml is read the way the
# release workflow itself reads it: the first version under [package].
source = open("Cargo.toml", encoding="utf-8").read()
package = re.search(r"^\[package\]$(.*?)(?=^\[|\Z)", source, re.M | re.S)
if package is None:
    raise SystemExit("Cargo.toml has no [package] section")
found = re.search(r'^version\s*=\s*"([^"]+)"', package.group(1), re.M)
if found is None:
    raise SystemExit("Cargo.toml names no package version")
version = found.group(1)

npm = json.load(open("npm/package.json", encoding="utf-8"))
if npm.get("version") != version:
    complaints.append(
        f"npm/package.json is {npm.get('version')} and Cargo.toml is {version}: "
        "the wrapper would fetch a release that is not this one"
    )

manifest = json.load(open("garden.json", encoding="utf-8"))
declared = manifest["install"]["binary"]["targets"]

ci = load(".github/workflows/ci.yml")
release = load(".github/workflows/release.yml")

# ci.yml runs the suite on a release build. The latency budget is measured on
# the release binary, so a debug-only run would let a regression through.
on_release = [
    (job, step)
    for job, _, step in steps(ci)
    if "cargo test" in runs(step) and re.search(r"(?<![-\w])--release(?![-\w])", runs(step))
]
if not on_release:
    complaints.append(
        ".github/workflows/ci.yml never runs `cargo test --release`: a latency regression "
        "measured on the release binary would not fail the build"
    )
for job, step in on_release:
    command = runs(step)
    if re.search(r"--(test|bin|lib|bench|example)\b", command):
        complaints.append(
            f"ci.yml job '{job}' narrows its release run to part of the suite (`{command.strip()}`): "
            "the latency test has to be inside what CI runs"
        )
    if (ci["jobs"][job].get("continue-on-error") or step.get("continue-on-error")):
        complaints.append(f"ci.yml job '{job}' continues on error, so nothing it runs can fail the build")

# The debug and the release runs cover the same suite; a release job that ran a
# smaller set would leave the budget unmeasured for whatever it skipped.
suites = {
    re.sub(r"\s*--release\b", "", runs(step)).strip()
    for _, _, step in steps(ci)
    if "cargo test" in runs(step)
}
if len(suites) != 1:
    complaints.append(
        "ci.yml runs different cargo test invocations on debug and release: "
        + " | ".join(sorted(suites))
    )

# release.yml carries the version check across the files a release publishes.
version_steps = [
    step
    for _, _, step in steps(release)
    if "Cargo.toml" in runs(step) and "npm/package.json" in runs(step)
]
if not version_steps:
    complaints.append(
        ".github/workflows/release.yml has no step reading Cargo.toml and npm/package.json: "
        "nothing stops a tag from publishing two different versions"
    )
else:
    checked = "\n".join(runs(step) for step in version_steps)
    if "GITHUB_REF" not in checked and "github.ref" not in checked:
        complaints.append("release.yml's version check never reads the tag it is releasing")
    if "exit 1" not in checked:
        complaints.append("release.yml's version check never leaves with a code, so a mismatch publishes anyway")

# The platform matrix is the manifest's targets, and no others.
matrix = set()
for _, job, _ in steps(release):
    for entry in ((job.get("strategy") or {}).get("matrix") or {}).get("include") or []:
        if "target" in entry:
            matrix.add(entry["target"])
if matrix != set(declared):
    complaints.append(
        "release.yml's build matrix and garden.json's install targets disagree, "
        f"only in the workflow: {sorted(matrix - set(declared))}; "
        f"only in the manifest: {sorted(set(declared) - matrix)}"
    )

# Every archive the manifest promises is an archive the workflow packages.
packaged = "\n".join(runs(step) for _, _, step in steps(release))
for target, asset in sorted(declared.items()):
    stem = asset.replace(target, "${{ matrix.target }}")
    if stem not in packaged:
        complaints.append(
            f"garden.json promises {asset} for {target}, and release.yml packages nothing named {stem}"
        )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"version {version} across Cargo.toml and npm/package.json; {len(declared)} targets in the matrix")
PY

if [ "$status" -eq 0 ]; then
  echo "the release pipeline: both workflows pass actionlint, the suite runs on a release build, the matrix is the manifest's"
fi
exit "$status"
