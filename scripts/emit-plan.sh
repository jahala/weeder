#!/usr/bin/env bash
# Emit weed's pleach plan from the map, the way this repo runs it:
# tend2 on PATH as the verifier (out of tree, LAW 1), scripts/check/run.sh as
# the evidence runner, casts and audits from plans/. Then patch the one thing
# tend2 emit-plan gets wrong today: the audit command omits the --runner
# template the smoke carries (docs/questions-tend2-pleach-2026-09-05.md, 8b).
set -euo pipefail
cd "$(dirname "$0")/.."
runner='bash scripts/check/run.sh {evidence}'
out="${1:-plans/weed.plan.json}"
tend2 emit-plan docs/tend2 --repo-root . --verify-bin tend2 --runner "$runner" \
  --cast-file plans/cast.json --audit-file plans/audit.json --node-timeout-ms 5400000 --out "$out"
jq --arg r "--runner '$runner'" \
  '.nodes |= map(if .accept.audit and (.accept.audit.command | endswith("--audit-egress")) then .accept.audit.command += " " + $r else . end)' \
  "$out" > "$out.tmp" && mv "$out.tmp" "$out"
pleach validate "$out" | jq -c '{valid, waves}'
