#!/usr/bin/env bash
# Evidence for check-face.tend2.html c7: the example plan gates on
# `weeder check --strict`, pleach accepts it, and docs/pleach.md explains why that
# command needs no base argument. Every flag the example cites is checked against
# the binary's own help, so the example cannot drift away from the CLI.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

plan="examples/pleach/plan.json"
doc="docs/pleach.md"
gate="weeder check --strict"
status=0

command -v pleach >/dev/null 2>&1 || {
  echo "pleach is not on PATH: the example cannot be validated against the contract it targets" >&2
  exit 3
}
command -v jq >/dev/null 2>&1 || { echo "jq is not on PATH" >&2; exit 3; }

[ -f "$plan" ] || { echo "$plan is missing: the example gate is unwritten" >&2; exit 1; }
[ -f "$doc" ] || { echo "$doc is missing: the gate is unexplained" >&2; exit 1; }

# The plan is a plan pleach will run.
if ! pleach validate "$plan" >/dev/null; then
  echo "$plan does not pass pleach validate" >&2
  status=1
fi

# Every node gates on weeder, with the flag that makes a worker's own suppression
# stop it, and with no base argument.
nodes="$(jq -r '.nodes | length' "$plan")"
[ "$nodes" -gt 0 ] || { echo "$plan declares no nodes" >&2; exit 1; }

gated="$(jq -r --arg gate "$gate" '[.nodes[] | select(.accept.smoke == $gate)] | length' "$plan")"
if [ "$gated" -ne "$nodes" ]; then
  echo "$plan: $gated of $nodes nodes declare \"accept\": { \"smoke\": \"$gate\" }" >&2
  status=1
fi

if jq -e '[.nodes[].accept.smoke // ""] | map(select(test("--base|--staged"))) | length > 0' "$plan" >/dev/null; then
  echo "$plan passes a base or a staged argument: the node worktree starts at HEAD, so weeder needs neither" >&2
  status=1
fi

# Every flag the example cites is a flag the binary has.
cargo build --quiet --bin weeder
binary="target/debug/weeder"
while IFS= read -r smoke; do
  set -- $smoke
  subcommand="$2"
  help="$("$binary" "$subcommand" --help 2>&1)" || {
    echo "$plan gates on '$smoke', but weeder has no '$subcommand' subcommand" >&2
    status=1
    continue
  }
  for flag in $(printf '%s\n' "$smoke" | tr ' ' '\n' | grep '^--' || true); do
    if ! printf '%s\n' "$help" | grep -q -- "$flag"; then
      echo "$plan gates on '$smoke', but weeder $subcommand has no $flag" >&2
      status=1
    fi
  done
done < <(jq -r '.nodes[].accept.smoke // empty' "$plan" | grep '^weeder ' | sort -u)

# The doc explains the gate, and what it compares against.
grep -qF "$gate" "$doc" || { echo "$doc never names the gate command \`$gate\`" >&2; status=1; }
grep -qF 'index plus the working tree against `HEAD`' "$doc" || {
  echo "$doc never says weeder judges the index plus the working tree against HEAD" >&2
  status=1
}
grep -qiE 'no base argument|needs no base argument|base argument would' "$doc" || {
  echo "$doc never explains why no base argument is needed" >&2
  status=1
}
grep -qF "$plan" "$doc" || { echo "$doc never points at $plan" >&2; status=1; }

if [ "$status" -eq 0 ]; then
  echo "examples/pleach/plan.json: $nodes nodes gated on '$gate', validated by pleach; docs/pleach.md explains the comparison"
fi
exit "$status"
