#!/usr/bin/env bash
# Evidence for windows: every unix-only line in this repository is gated, and
# the gate says what Windows has instead.
#
# The release contract names five platforms and v0.1.0 shipped four, because the
# library imported `std::os::unix::fs::PermissionsExt` with nothing around it and
# the tag was the first time anything was compiled for Windows. A grep would
# catch that import; it would not catch the next one, which is why this reads the
# shape of every such line rather than a list of the ones somebody remembered.
#
# The rule is one sentence: a line that reaches for a platform-only module of the
# standard library sits inside a `cfg(unix)` gate, and that gate either has a
# `cfg(windows)` twin under the same name or carries a comment beside it saying
# what Windows lacks. The two helpers `guard` installs a hook with are held to
# both halves: each has a unix meaning and a Windows one, and the reason is
# written above them.
#
# What no static reading can settle is whether the tree compiles and passes over
# there; that is `scripts/check/windows-ci.sh`, which reads the run itself. So
# this ends by running the suite on the machine it is on, which is the other half
# of a gate being right: a `cfg(unix)` arm that stopped compiling here would be
# just as broken as a Windows one nobody built.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the sources cannot be read. The check refuses to pass on unread code." >&2
  exit 3
}
command -v cargo >/dev/null 2>&1 || {
  echo "cargo is not on PATH, so the suite cannot be run. The check refuses to pass on an unrun suite." >&2
  exit 3
}

status=0

python3 - <<'PY' || status=1
"""Every platform-only line in src/, xtask/ and tests/, and the gate over it.

The reading is the compiler's own, taken off the formatting `cargo fmt` holds:
an item's attributes sit directly above it at the item's indentation, and an
item ends where the indentation comes back. A gate therefore covers the lines
from its attribute down to the first line that returns to the attribute's own
column, which is one statement for a gated statement and one whole item for a
gated item.
"""

import pathlib
import re
import sys

# The standard library's platform-only modules. `std::os::unix` is the one that
# broke the release; the others are here because a gate that only knows one name
# is a gate the next line walks around.
PLATFORM_MODULE = re.compile(r"\bstd::os::(unix|fd|linux|macos|wasi)\b")
# The gates, and the platform each names.
GATE = re.compile(r"#!?\[cfg(_attr)?\(.*?\b(unix|windows|target_family|target_os)\b")
UNIX_GATE = re.compile(r"#!?\[cfg(_attr)?\([^)]*\bunix\b")
WINDOWS_GATE = re.compile(r"#!?\[cfg(_attr)?\([^)]*\bwindows\b")
# What a gated item is called, so a twin can be recognised under the same name.
NAMED = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const|static|unsafe\s+)?"
    r"(?:fn|struct|enum|trait|mod|type|use)\s+([A-Za-z_][A-Za-z0-9_]*)"
)
COMMENT = re.compile(r"^\s*(//|/\*|\*)")
# The two helpers whose meaning differs per platform rather than being absent on
# one: a hook git will run, and the mode that makes it one.
HOOK_HELPERS = ("is_executable", "make_executable")
HELPER_FILE = "src/seams/fs.rs"

WHERE = ("src", "xtask", "tests")


def indent(line):
    return len(line) - len(line.lstrip())


def sources():
    for directory in WHERE:
        for path in sorted(pathlib.Path(directory).rglob("*.rs")):
            yield path


def gates(lines):
    """Every gate in a file, as (first line, last line, platform, indent).

    Line numbers are 0-based and both ends are inside the region the gate
    covers. A file-level `#![cfg(...)]` covers the whole file.
    """
    found = []
    for at, line in enumerate(lines):
        if not GATE.search(line):
            continue
        column = indent(line)
        if line.lstrip().startswith("#!["):
            found.append((0, len(lines) - 1, line, column))
            continue
        # Past the attribute, past any further attributes and comments, is the
        # header the gate is written for; the region ends where the indentation
        # comes back to the attribute's own column.
        after = at + 1
        while after < len(lines) and (
            lines[after].lstrip().startswith("#[") or COMMENT.match(lines[after])
        ):
            after += 1
        last = len(lines) - 1
        for later in range(after + 1, len(lines)):
            if not lines[later].strip():
                continue
            if indent(lines[later]) <= column:
                last = later if lines[later].lstrip().startswith("}") else later - 1
                break
        found.append((at, last, line, column))
    return found


def reason(lines, at):
    """The comment block written directly above a line, as one string."""
    written = []
    above = at - 1
    while above >= 0 and COMMENT.match(lines[above]):
        written.append(lines[above].strip())
        above -= 1
    return "\n".join(reversed(written))


def name_at(lines, at):
    """The name of the item a gate at `at` is written for, where it names one."""
    for later in range(at + 1, min(at + 8, len(lines))):
        if lines[later].lstrip().startswith("#[") or COMMENT.match(lines[later]):
            continue
        found = NAMED.match(lines[later])
        return found.group(1) if found else None
    return None


complaints = []
gated = 0

for path in sources():
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    found = gates(lines)
    windows_names = {
        name_at(lines, first)
        for first, _, line, _ in found
        if WINDOWS_GATE.search(line) and name_at(lines, first)
    }
    for at, line in enumerate(lines):
        if not PLATFORM_MODULE.search(line):
            continue
        # A mention inside a comment is prose about the gate, not a use of it.
        if COMMENT.match(line):
            continue
        covering = [
            (first, last, gate, column)
            for first, last, gate, column in found
            if first <= at <= last and UNIX_GATE.search(gate)
        ]
        if not covering:
            complaints.append(
                f"{path}:{at + 1} reaches for a platform-only module with no cfg(unix) gate "
                f"over it, which is the shape that let the windows leg of v0.1.0 fail at the "
                f"tag: {line.strip()}"
            )
            continue
        # The innermost gate is the one that speaks for this line.
        first, _, gate, _ = max(covering, key=lambda region: region[0])
        gated += 1
        named = name_at(lines, first)
        said = reason(lines, first)
        if named in windows_names:
            continue
        if not re.search(r"\bwindows\b", said, re.IGNORECASE):
            complaints.append(
                f"{path}:{at + 1} is gated at line {first + 1} with neither a cfg(windows) twin "
                f"nor a reason naming what Windows has instead. A gate whose reason nobody wrote "
                f"is a platform quietly dropped."
            )

# The two helpers a hook is installed with have a meaning on both platforms
# rather than being absent on one, so each is written twice and the passage that
# settles it stands above them.
helper = pathlib.Path(HELPER_FILE)
if not helper.is_file():
    complaints.append(f"{HELPER_FILE} is missing, and it is where the hook helpers live")
else:
    lines = helper.read_text(encoding="utf-8").splitlines()
    found = gates(lines)
    for name in HOOK_HELPERS:
        for platform, pattern in (("unix", UNIX_GATE), ("windows", WINDOWS_GATE)):
            written = [
                first
                for first, _, gate, _ in found
                if pattern.search(gate) and name_at(lines, first) == name
            ]
            if len(written) != 1:
                complaints.append(
                    f"{HELPER_FILE} carries {len(written)} cfg({platform}) definitions of {name}, "
                    f"and the question it answers has one answer per platform"
                )
                continue
            if not reason(lines, written[0]).strip():
                complaints.append(
                    f"{HELPER_FILE}: the cfg({platform}) {name} carries no comment saying what it "
                    f"means there, and the two meanings are the whole of why it is written twice"
                )
    said = helper.read_text(encoding="utf-8")
    passage = said[: said.index("#[cfg")] if "#[cfg" in said else ""
    for word in ("windows", "git", "mode"):
        if not re.search(rf"\b{word}", passage, re.IGNORECASE):
            complaints.append(
                f"{HELPER_FILE}: the reason above the hook helpers never says {word}, and what "
                f"makes a file a hook is git's answer on each platform rather than weeder's"
            )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"{gated} platform-only lines, every one gated and every gate answered for")
PY

if [ "$status" -ne 0 ]; then
  exit "$status"
fi

# The other half of a gate being right: the arms this machine compiles are the
# ones it runs. A cfg arm that stopped compiling here would be as broken as a
# Windows one nobody built, so the whole suite is built and run.
#
# `cargo test` runs one test binary after another, and this suite is most of
# three minutes of that, past the budget a verifier gives one check. So the
# binaries are built with the invocation CI runs and then started together,
# every one of them, and every exit code is read. It is the same suite in a
# different order.
echo "the host suite, built the way CI builds it and run all at once"
cargo test --workspace --no-run --quiet || exit 1
cargo test --workspace --doc --quiet || status=1

python3 - <<'SUITE' || status=1
"""Every test binary the workspace builds, run at the same time."""

import concurrent.futures
import json
import os
import subprocess
import sys

built = subprocess.run(
    ["cargo", "test", "--workspace", "--no-run", "--message-format=json"],
    capture_output=True,
    text=True,
)
if built.returncode != 0:
    print(built.stderr, file=sys.stderr)
    raise SystemExit(f"the suite did not build, so nothing ran: exit {built.returncode}")

binaries = []
for line in built.stdout.splitlines():
    try:
        message = json.loads(line)
    except ValueError:
        continue
    if message.get("reason") != "compiler-artifact":
        continue
    if not message.get("executable"):
        continue
    if not (message.get("profile") or {}).get("test"):
        continue
    binaries.append(message["executable"])

if not binaries:
    raise SystemExit(
        "the workspace built no test binaries, and a suite nobody ran proves nothing"
    )


def run(binary):
    done = subprocess.run([binary, "--quiet"], capture_output=True, text=True)
    return binary, done.returncode, done.stdout + done.stderr


# One process per binary is what makes this fit, and the pool is held to half
# the machine because a binary already runs its own cases on threads: a suite
# run under a load the machine cannot carry starts failing on its own deadlines,
# which is a measurement of the load rather than of the code.
workers = max(4, (os.cpu_count() or 8) // 2)
failed = []
with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
    for binary, code, said in pool.map(run, binaries):
        if code != 0:
            failed.append((binary, code, said))

# A binary that failed is run again on its own, and it is that run which counts.
# A test that is really broken fails alone; one that lost a race with the rest of
# the suite for the machine passes, and saying otherwise would be a red nobody
# could reproduce.
confirmed = []
for binary, _, _ in failed:
    binary, code, said = run(binary)
    if code != 0:
        confirmed.append(f"{os.path.basename(binary)} left with {code}\n{said}")

for failure in confirmed:
    print(failure, file=sys.stderr)
if confirmed:
    raise SystemExit(f"{len(confirmed)} of {len(binaries)} test binaries failed")
if failed:
    print(
        f"{len(failed)} of {len(binaries)} test binaries failed under the parallel load and "
        f"passed on their own"
    )
print(f"{len(binaries)} test binaries, every one green")
SUITE

if [ "$status" -eq 0 ]; then
  echo "every std::os::unix line is gated and answered for, the two hook helpers carry both platform meanings, and the host suite is green"
fi
exit "$status"
