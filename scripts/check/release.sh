#!/usr/bin/env bash
# Evidence for garden-fit: the release pipeline, in tilth's shape, publishing
# the artifact the garden's lockfile pins.
#
# Nothing here is run against GitHub, that is the owner's action, so the
# workflows are proved the only ways they can be proved on this machine: they
# parse and pass actionlint, and the claims that matter are read out of the yaml
# and checked against the repository they release. A workflow that names a
# target the manifest does not, or a version the crate does not, is a release
# that goes wrong at the one moment nobody is watching.
#
# The artifact's shape is not read, it is built: the packaging step the workflow
# runs is run here for this machine's own platform, and the tarball it writes is
# opened and its digest file checked. A shape nobody has unpacked is a promise.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

status=0

for file in .github/workflows/ci.yml .github/workflows/release.yml scripts/package-release.sh \
  npm/package.json npm/install.js npm/run.js; do
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
command -v node >/dev/null 2>&1 || {
  echo "node is not on PATH, so the npm wrapper cannot be asked what it fetches. The check refuses to pass on an unread wrapper." >&2
  exit 3
}
command -v cargo >/dev/null 2>&1 || {
  echo "cargo is not on PATH, so nothing can be built to package. The check refuses to pass on an unpackaged artifact." >&2
  exit 3
}

if ! actionlint .github/workflows/ci.yml .github/workflows/release.yml; then
  echo "actionlint refused the workflows" >&2
  status=1
fi

python3 - <<'PY' || status=1
import fnmatch
import json
import re
import sys

import yaml


def load(path):
    with open(path, encoding="utf-8") as handle:
        return yaml.safe_load(handle)


def triggers(workflow):
    """What the workflow runs on. YAML reads a bare `on:` key as the boolean."""
    return workflow.get("on") or workflow.get(True) or {}


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
declared = manifest["install"]["binaries"]
name = manifest["name"]
skill = manifest["faces"]["skill"]
binary = manifest["install"].get("binary_name", manifest["faces"]["cli"])

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

# The release runs on a tag and on nothing else: a pipeline that publishes from
# a branch push publishes whatever was on the branch.
tags = ((triggers(release).get("push") or {}).get("tags")) or []
if not tags:
    complaints.append(
        ".github/workflows/release.yml does not run on a tag push, so nothing publishes a release"
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

# Every archive the manifest promises is the archive the contract names: one
# gzipped tar per platform, carrying the executable, the manifest and the skill.
for target, url in sorted(declared.items()):
    expected = f"/releases/download/v{version}/{name}-{target}.tar.gz"
    if not url.endswith(expected):
        complaints.append(
            f"garden.json points {target} at {url}, and the release publishes "
            f"{name}-{target}.tar.gz for v{version}"
        )

# One packaging step, on every platform, running the script that names the
# artifact out of the manifest. A step that packages a subset, or packages by
# hand beside the script, is a shape nothing here has proved.
packaging = [
    (job, step)
    for job, _, step in steps(release)
    if "scripts/package-release.sh" in runs(step)
]
if len(packaging) != 1:
    complaints.append(
        f"release.yml runs the packaging script in {len(packaging)} steps: the artifact has one "
        "shape, written in one place, or the platforms drift apart"
    )
else:
    job, step = packaging[0]
    command = runs(step)
    if "${{ matrix.target }}" not in command:
        complaints.append(
            "release.yml's packaging step does not pass ${{ matrix.target }}, so every leg of the "
            "matrix would name its asset the same thing"
        )
    if step.get("if"):
        complaints.append(
            f"release.yml's packaging step carries `if: {step['if']}`, so some platform of the "
            "matrix publishes nothing"
        )
    if step.get("shell") != "bash":
        complaints.append(
            "release.yml's packaging step does not name bash, and the windows runner would hand "
            "the script to PowerShell"
        )
for _, _, step in steps(release):
    if re.search(r"\b(7z|zip)\b", runs(step)):
        complaints.append(
            f"release.yml still packages with `{runs(step).strip().splitlines()[0]}`: the contract "
            "is one tarball per platform, windows included"
        )

# The digest travels with the asset, or the lockfile has nothing to pin.
uploaded = []
for _, _, step in steps(release):
    if "action-gh-release" in (step.get("uses") or ""):
        uploaded += (step.get("with") or {}).get("files", "").split()
for target in sorted(declared):
    for asset in (f"{name}-{target}.tar.gz", f"{name}-{target}.tar.gz.sha256"):
        if not any(fnmatch.fnmatch(asset, pattern) for pattern in uploaded):
            complaints.append(
                f"release.yml uploads {uploaded or 'nothing'}, which does not carry {asset}"
            )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(
    f"version {version} across Cargo.toml and npm/package.json; {len(declared)} targets in the "
    f"matrix; every asset published with its digest"
)
PY

# --- the packaging step, run ---------------------------------------------------
#
# The workflow's own script, for this machine's platform, into a directory
# outside the tree. What it writes is unpacked and its digest verified, so the
# claim about the artifact's shape is a claim somebody has opened.
node -e '
  const manifest = require("./garden.json");
  const install = require("./npm/install.js");

  const declared = Object.keys(manifest.install.binaries).sort();
  const mapped = Object.values(install.PLATFORM_MAP).sort();
  const complaints = [];
  if (JSON.stringify(declared) !== JSON.stringify(mapped)) {
    complaints.push(
      `npm/install.js maps ${mapped.join(", ")} and garden.json publishes ${declared.join(", ")}`
    );
  }
  for (const target of declared) {
    const wrapper = install.assetUrl(target, manifest.version);
    if (wrapper !== manifest.install.binaries[target]) {
      complaints.push(
        `npm/install.js fetches ${wrapper} for ${target}, and garden.json publishes ` +
          manifest.install.binaries[target]
      );
    }
  }
  for (const complaint of complaints) console.error(complaint);
  if (complaints.length) process.exit(1);
  console.log(`the npm wrapper fetches the ${declared.length} assets garden.json publishes`);
' || status=1

target="$(rustc -vV | sed -n 's/^host: //p')"
[ -n "$target" ] || { echo "rustc names no host triple, so nothing can be packaged for this machine" >&2; exit 3; }

echo "packaging $target, the way the workflow packages every platform"
cargo build --release --locked --quiet
out="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$out"' EXIT

if ! bash scripts/package-release.sh "$target" target/release "$out"; then
  echo "the packaging step did not finish, so no release artifact exists to judge" >&2
  exit 1
fi

asset="$(node -e 'process.stdout.write(require("./npm/install.js").assetName(process.argv[1]))' "$target")"
python3 - "$out" "$asset" <<'PY' || status=1
import json
import os
import subprocess
import sys
import tarfile

out, asset = sys.argv[1], sys.argv[2]
manifest = json.load(open("garden.json", encoding="utf-8"))
skill = manifest["faces"]["skill"]
binary = manifest["install"].get("binary_name", manifest["faces"]["cli"])

complaints = []
archive = f"{out}/{asset}"

if not os.path.exists(archive):
    raise SystemExit(
        f"the packaging step wrote {sorted(os.listdir(out))} and the wrapper fetches {asset}: "
        "an asset nobody publishes under that name installs as a 404"
    )

with tarfile.open(archive) as tar:
    members = {member.name for member in tar.getmembers() if member.isfile()}
    wanted = {binary, "garden.json", skill}
    for path in sorted(wanted - members):
        complaints.append(f"{asset} does not carry {path} at its top level: {sorted(members)}")
    for path in sorted(members - wanted):
        complaints.append(f"{asset} carries {path}, which the artifact contract does not name")
    if binary in members:
        mode = tar.getmember(binary).mode
        if not mode & 0o111:
            complaints.append(f"{asset} carries {binary} unexecutable ({mode:o})")
    if "garden.json" in members:
        packed = json.load(tar.extractfile("garden.json"))
        if packed != manifest:
            complaints.append(
                f"{asset} carries a garden.json that is not this repository's: the stem reads the "
                "manifest out of the verified artifact and would read the wrong one"
            )

digest = f"{archive}.sha256"
try:
    stated = open(digest, encoding="utf-8").read().split()
except FileNotFoundError:
    complaints.append(f"{asset}.sha256 was not written, so the lockfile has no digest to pin")
    stated = []
if stated:
    if len(stated) < 2 or stated[1].lstrip("*") != asset:
        complaints.append(f"{asset}.sha256 names {stated[1:]}, not {asset}")
    computed = subprocess.run(
        ["shasum", "-a", "256", archive], capture_output=True, text=True, check=True
    ).stdout.split()[0]
    if stated[0] != computed:
        complaints.append(
            f"{asset}.sha256 states {stated[0]} and the tarball hashes to {computed}: a digest "
            "that does not match refuses the judge it was meant to pin"
        )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"{asset} carries {binary}, garden.json and {skill}, and its digest file matches")
PY

if [ "$status" -eq 0 ]; then
  echo "the release pipeline: both workflows pass actionlint, the suite runs on a release build, the matrix is the manifest's, and the packaged artifact is the shape the lock pins"
fi
exit "$status"
