#!/usr/bin/env bash
# Evidence for bite c3, the kill bar: `docs/bite-2026-09.md` shows at least 90
# percent of the proof repository's phased nodes yielding a clean test commit,
# or it records the kill and `weeder --help` does not print `bite`.
#
# Nothing here trusts a number the file printed. The proof repository is weeder
# itself, built by a conductor node by node, so the ground truth is in two
# places the file cannot edit: `plans/`, the work orders the conductor was
# given, which say which nodes asked for phases, and the commits those nodes
# landed, which say what came back. Every row of the file is counted against
# those, the totals are recomputed from the rows, and the share is recomputed
# from the totals before it is put against the bar.
#
# Then the consequence is held to the measurement, in both directions. Over the
# bar, `bite` has to be a face `weeder --help` offers. Under it, the file has to
# record the kill in its first sentence and `weeder --help` must not print `bite`,
# so nobody can ship the face by editing prose, and nobody can leave the face
# hidden once the measurement says it earned its place.
#
# The report and the plans are overridable so a suite can run this script
# against a measurement it wrote itself; with nothing set it reads what the
# repository ships.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

report="${WEEDER_BITE_REPORT:-docs/bite-2026-09.md}"
plans="${WEEDER_BITE_PLANS:-plans}"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the report cannot be added up. The check refuses to pass on unchecked arithmetic." >&2
  exit 3
}
command -v git >/dev/null 2>&1 || {
  echo "git is not on PATH, so the commits the nodes landed cannot be read." >&2
  exit 3
}
command -v cargo >/dev/null 2>&1 || {
  echo "cargo is not on PATH, so the binary cannot be asked which faces it offers." >&2
  exit 3
}
[ -f "$report" ] || {
  echo "$report is missing: there is no measurement to read a bar off, and the kill is not recorded anywhere." >&2
  exit 1
}

# The faces are read off the binary this tree builds, never off a file beside
# it: what weeder offers is what weeder prints.
cargo build --quiet
printed="$(./target/debug/weeder --help)"

python3 - "$report" "$plans" "$printed" <<'PY'
import json
import os
import re
import subprocess
import sys

report_path, plans_path, printed = sys.argv[1:4]
report = open(report_path, encoding="utf-8").read()
complaints = []

BAR = 90.0
# The conductor's own subject line for the commit a settled node lands.
LANDED = re.compile(r"^pleach: (?P<node>\S+) (verified|quarantined)")
# What a path has to look like to be a test, as `src/core/classify.rs` reads
# one. The measurement asks which commit carried the tests, so it has to answer
# the same question weeder's classifier answers, and this is that question in the
# only form a shell script can ask it.
TEST_PATH = re.compile(
    r"(^|/)tests?/|(^|/)__tests__/|\.test\.|\.spec\.|(^|/)test_[^/]*\.py$"
    r"|_test\.(py|go|rs)$|(^|/)tests\.rs$"
)


def git(*arguments):
    """git, asked a question, with its answer as lines."""
    done = subprocess.run(
        ["git", *arguments], capture_output=True, text=True, check=False
    )
    if done.returncode != 0:
        return []
    return [line for line in done.stdout.splitlines() if line.strip()]


def work_shape(node):
    """Which of the conductor's three work shapes a node asked for."""
    work = node.get("work", {})
    if "phases" in work:
        return "phases"
    if "command" in work:
        return "command"
    return "prompt"


# ── the plans: which nodes were dispatched, and which asked for phases ────────
dispatched = {}
if not os.path.isdir(plans_path):
    complaints.append(f"{plans_path} is not there, so no node can be counted")
for name in sorted(os.listdir(plans_path) if os.path.isdir(plans_path) else []):
    if not name.endswith(".json"):
        continue
    try:
        plan = json.load(open(os.path.join(plans_path, name), encoding="utf-8"))
    except (OSError, ValueError) as error:
        complaints.append(f"{name} is in {plans_path} and does not read as json: {error}")
        continue
    if not isinstance(plan, dict) or "nodes" not in plan:
        continue
    for node in plan["nodes"]:
        entry = dispatched.setdefault(
            node["id"], {"plans": set(), "shape": work_shape(node)}
        )
        entry["plans"].add(name)
        # A node re-emitted under a second plan with a different shape is
        # counted as phased if either plan asked for phases: the question is
        # whether the conductor was ever asked to run it in phases.
        if work_shape(node) == "phases":
            entry["shape"] = "phases"

# ── the commits: what each node landed ───────────────────────────────────────
# Branches, remotes and tags are where a landing lives. `--all` would also walk
# the stash and any tool's private refs, and a stash taken on a history that
# was since rewritten carries the old landings under old shas, counting each
# one twice on the machine that holds it.
landings = {}
for line in git("log", "--branches", "--remotes", "--tags", "--format=%H%x09%s"):
    sha, _, subject = line.partition("\t")
    found = LANDED.match(subject)
    if found:
        landings.setdefault(found.group("node"), []).append(sha)


def paths_in(sha, parent):
    return git("diff", "--name-only", parent, sha)


def yields_test_commit(node):
    """Whether the node landed its tests apart from its implementation.

    The commit a node settles with is the last of the ones it landed. Where it
    carries test files and production files together, the phases went in as one
    commit and there is nothing bite can apply alone. Where it carries no tests
    at all, the commit under it is asked the same question, which is the shape a
    conductor that commits its test phase separately leaves behind.
    """
    settled = landings.get(node, [])
    if not settled:
        return False, "landed no commit in this history"
    sha = settled[0]
    parents = git("rev-parse", f"{sha}^")
    if not parents:
        return False, f"{sha[:9]} has no commit under it to be applied to"
    changed = paths_in(sha, parents[0])
    tests = [path for path in changed if TEST_PATH.search(path)]
    rest = [path for path in changed if not TEST_PATH.search(path)]
    if tests and rest:
        return False, f"{sha[:9]} carries the tests and the implementation together"
    if tests and not rest:
        return False, f"{sha[:9]} is the whole node, and it carries tests alone"
    below = parents[0]
    under = git("rev-parse", f"{below}^")
    if not under:
        return False, f"{below[:9]} has no commit under it to be applied to"
    beneath = paths_in(below, under[0])
    if beneath and all(TEST_PATH.search(path) for path in beneath):
        return True, f"{below[:9]} carries the tests alone, under {sha[:9]}"
    return False, f"no commit under {sha[:9]} carries the tests alone"


measured = {}
for node, entry in dispatched.items():
    clean, why = (False, "not a phased node")
    if entry["shape"] == "phases":
        clean, why = yields_test_commit(node)
    measured[node] = {
        "plans": sorted(entry["plans"]),
        "shape": entry["shape"],
        "landed": len(landings.get(node, [])),
        "clean": clean,
        "why": why,
    }

nodes = len(measured)
phased = sum(1 for entry in measured.values() if entry["shape"] == "phases")
clean = sum(1 for entry in measured.values() if entry["clean"])
share = None if phased == 0 else round(clean * 100.0 / phased, 2)

# ── the file, read row by row against what was measured ──────────────────────
rows = {}
for row in re.finditer(
    r"^\| `(?P<node>[^`]+)` \|[^|]*\| (?P<shape>\w+) \| (?P<landed>\d+) \| (?P<clean>yes|no) \|",
    report,
    re.M,
):
    rows[row.group("node")] = {
        "shape": row.group("shape"),
        "landed": int(row.group("landed")),
        "clean": row.group("clean") == "yes",
    }

for node, entry in sorted(measured.items()):
    printed_row = rows.get(node)
    if printed_row is None:
        complaints.append(f"{node} ran and {report_path} carries no row for it")
        continue
    for field in ("shape", "landed", "clean"):
        if printed_row[field] != entry[field]:
            complaints.append(
                f"{node}: the file says {field}={printed_row[field]!r} and the repository says {entry[field]!r}"
            )
for node in rows:
    if node not in measured:
        complaints.append(f"{report_path} carries a row for {node}, which no plan dispatched")

# The totals, recomputed from the rows rather than read.
totals = re.search(
    r"^\| \*\*pooled\*\* \| \*\*(?P<nodes>\d+)\*\* \| \*\*(?P<phased>\d+)\*\* \| \*\*(?P<clean>\d+)\*\* \| \*\*(?P<share>[\d.]+%|none)\*\* \|",
    report,
    re.M,
)
if totals is None:
    complaints.append(f"{report_path} carries no pooled row, so there is no share to read")
else:
    for field, counted in (("nodes", nodes), ("phased", phased), ("clean", clean)):
        if int(totals.group(field)) != counted:
            complaints.append(
                f"the pooled row says {field}={totals.group(field)} and the nodes add up to {counted}"
            )
    written = totals.group("share")
    expected = "none" if share is None else f"{share}%"
    if written != expected:
        complaints.append(f"the pooled share printed is {written} and the numbers give {expected}")

# ── the consequence, held to the measurement in both directions ──────────────
def commands(text):
    """The subcommands a help listing offers, read the way a person reads them:
    the indented lines under `Commands:`, up to the blank line that ends the
    section. `help` is the parser's own furniture rather than a face of weeder."""
    listed = []
    inside = False
    for line in text.splitlines():
        if line.strip() == "Commands:":
            inside = True
            continue
        if inside:
            if not line.strip():
                break
            listed.append(line.split()[0])
    return [name for name in listed if name != "help"]


offered = "bite" in commands(printed)

KILL = "bite does not ship:"
killed = next((line for line in report.splitlines()[1:] if line.strip()), "").startswith(KILL)
met = share is not None and share >= BAR

if met:
    if not offered:
        complaints.append(
            f"{clean} of {phased} phased nodes yield a clean test commit, at or over the "
            f"{BAR:.0f}% bar, and `weeder --help` does not offer bite. the face was measured "
            "into the release; take the hide off the subcommand."
        )
    if killed:
        complaints.append(
            f"the measurement clears the bar and {report_path} still opens with `{KILL}`"
        )
else:
    if not killed:
        complaints.append(
            f"the measurement is {clean} of {phased} phased nodes, under the {BAR:.0f}% bar, "
            f"and {report_path} does not open with `{KILL}`. the kill is the outcome, and the "
            "file has to say so in its first sentence."
        )
    if offered:
        complaints.append(
            "the measurement does not clear the bar and `weeder --help` offers bite. "
            "an unshipped face is one the binary does not print."
        )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)

print(
    f"{clean} of {phased} phased nodes of {nodes} yield a clean test commit"
    + (f" ({share}%)" if share is not None else "")
    + (
        f", at or over the {BAR:.0f}% bar, and weeder --help offers bite"
        if met
        else f", under the {BAR:.0f}% bar; {report_path} records the kill and weeder --help does not offer bite"
    )
)
PY
