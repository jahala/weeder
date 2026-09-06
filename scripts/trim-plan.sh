#!/usr/bin/env bash
# One node of the emitted plan as a plan of its own, so siblings can run in
# parallel under separate pleach locks: the node's needs are cleared (the
# conductor dispatches in dependency order by hand) and the source names the
# loop, which is what pleach keys its lock on.
#
# The emitted prompt tells a worker that `.loop-scratch/` is never collected.
# Under pleach 0.0.1 it is collected, judged by the hygiene gate, and a path the
# editor touched outside the worktree kills the delivery (dogfood P8, P10, P11),
# so that sentence is replaced with the truth before the worker reads it.
set -euo pipefail
id="${1:?loop id required}"
plan="${2:-plans/weeder.plan.json}"
out="plans/weeder.$id.plan.json"
python3 - "$id" "$plan" "$out" <<'PY'
import json, sys
id, plan, out = sys.argv[1:4]
doc = json.load(open(plan))
nodes = [node for node in doc["nodes"] if node["id"] == id]
if not nodes:
    raise SystemExit(f"{plan} has no node {id}")
node = nodes[0]
node["needs"] = []
doc["nodes"] = [node]
doc["source"] = f"docs/tend2#{id}"
old = ("- Probe/scratch files (toolchain checks, experiments) go in `.loop-scratch/` only — "
       "it is never collected; scratch files elsewhere pollute the delivery.")
new = ("- Scratch (probes, experiments, packets, session logs) goes under `$TMPDIR`, written through "
       "the shell only, never with the Write or Edit tool, and never under `.loop-scratch/` or "
       "anywhere in this worktree: the collector stages every path the editor touched, a path "
       "outside the worktree kills the delivery, and the hygiene gate judges whatever is left in "
       "the tree, including its secret scan.")
prompt = node["work"]["prompt"]
if old not in prompt:
    raise SystemExit("the emitted prompt no longer carries the scratch sentence this script replaces; read it and update the script")
node["work"]["prompt"] = prompt.replace(old, new, 1)
json.dump(doc, open(out, "w"), indent=2)
open(out, "a").write("\n")
print(out)
PY
