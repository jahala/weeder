#!/usr/bin/env bash
# Evidence for hooks.tend2.html c4: a real Claude Code session, with weed on the
# Stop hook, refuses a premature done and says why in weed's own words — and
# ends on its own once the tree is clean.
#
# Nothing here is simulated. The session is started with `claude -p`, the hook is
# handed to it with `--settings`, and what the script asserts on is the
# transcript that session wrote. The capture goes into docs/proof-2026-09.md
# under a dated heading, replacing the one that is there, so running this twice
# does not grow the file.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

doc="docs/proof-2026-09.md"
heading="## claude code — the stop hook, run for real"
# The model the proof runs on. It is a proof of the hook, not of the model, so
# the cheapest one that can read a sentence will do.
model="${WEED_PROOF_MODEL:-haiku}"
# How long the refused half is given to show the block before it is stopped. It
# never ends on its own: that is what is being proven.
limit="${WEED_PROOF_LIMIT:-90}"
marker="weed hook refused"

for tool in claude git jq; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool is not on PATH: the proof needs a real session, not a stand-in" >&2
    exit 3
  }
done
[ -f "$doc" ] || { echo "$doc is missing: there is nowhere to record the run" >&2; exit 3; }

cargo build --quiet --bin weed
weed="$root/target/debug/weed"

work="$(mktemp -d)"
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
git -C "$work" init -q --initial-branch=main
git -C "$work" -c user.name="weed proof" -c user.email="proof@weed.invalid" \
  commit -q --allow-empty -m "the repository begins"

resolved() {
  printf 'export function parse(input: string): string[] {\n  return input.split(";");\n}\n'
}
# The half-finished merge, built a character at a time so this script never
# opens a line with a conflict marker of its own.
conflicted() {
  local ours theirs separator
  ours="$(printf '<%.0s' 1 2 3 4 5 6 7)"
  separator="$(printf '=%.0s' 1 2 3 4 5 6 7)"
  theirs="$(printf '>%.0s' 1 2 3 4 5 6 7)"
  printf 'export function parse(input: string): string[] {\n'
  printf '%s HEAD\n  return input.split(",");\n' "$ours"
  printf '%s\n  return input.split(";");\n' "$separator"
  printf '%s feature/split-on-semicolons\n}\n' "$theirs"
}

mkdir -p "$work/src"
resolved > "$work/src/parser.ts"
git -C "$work" add -A
git -C "$work" -c user.name="weed proof" -c user.email="proof@weed.invalid" \
  commit -q -m "the parser"

cat > "$work/settings.json" <<JSON
{"hooks": {"Stop": [{"matcher": "", "hooks": [{"type": "command", "command": "$weed hook claude"}]}]}}
JSON

# A session started from inside another one refuses to run while CLAUDECODE is
# set, so the proof starts claude with that variable unset.
session() { # $1 transcript, $2 seconds before it is stopped, $3 whether to wait for the marker
  local transcript="$1" seconds="$2" until_blocked="$3" pid waited=0
  ( cd "$work" && env -u CLAUDECODE -u CLAUDE_CODE_ENTRYPOINT claude \
      -p "Reply with exactly: done. Change nothing." \
      --settings "$work/settings.json" \
      --output-format stream-json --verbose --restricted \
      --model "$model" ) > "$transcript" 2>&1 &
  pid=$!
  while kill -0 "$pid" 2>/dev/null && [ "$waited" -lt "$seconds" ]; do
    if [ "$until_blocked" = "blocked" ] && grep -q "$marker" "$transcript"; then
      break
    fi
    sleep 1
    waited=$((waited + 1))
  done
  if kill -0 "$pid" 2>/dev/null; then
    kill -TERM "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    return 124
  fi
  wait "$pid"
}

status=0

# The refused half: the tree carries a half-finished merge.
conflicted > "$work/src/parser.ts"
refused_table="$( (cd "$work" && "$weed" check --strict --format table) || true )"
session "$work/refused.jsonl" "$limit" blocked || true

blocks="$(grep -c "$marker" "$work/refused.jsonl" || true)"
if [ "$blocks" -eq 0 ]; then
  echo "the session ended over a tree weed refuses, and said nothing about it" >&2
  status=1
fi
feedback="$(jq -r 'select(.type=="user") | .message.content[]? | select(.type=="text") | .text' \
  "$work/refused.jsonl" 2>/dev/null | grep -A20 "Stop hook feedback" | head -20 || true)"
if ! printf '%s' "$feedback" | grep -q "$marker"; then
  echo "the session was blocked, but weed's reason never reached it" >&2
  status=1
fi
if ! printf '%s' "$feedback" | grep -q 'G1'; then
  echo "the reason the session received names no finding" >&2
  status=1
fi

# The allowed half: the merge is finished.
resolved > "$work/src/parser.ts"
allowed_table="$( (cd "$work" && "$weed" check --strict --format table) || true )"
allowed_code=0
session "$work/allowed.jsonl" "$limit" ends || allowed_code=$?

if [ "$allowed_code" -ne 0 ]; then
  echo "the session could not end over a clean tree: it left with $allowed_code" >&2
  status=1
fi
if grep -q "$marker" "$work/allowed.jsonl"; then
  echo "weed blocked a stop over a tree it has nothing against" >&2
  status=1
fi

if [ "$status" -ne 0 ]; then
  echo "the transcripts are at $work" >&2
  exit "$status"
fi

# What happened, written where the loop cites it.
capture="$work/capture.md"
{
  echo "### $(date +%Y-%m-%d) — $(claude --version | head -1), $("$weed" --version)"
  echo
  echo "Both halves ran \`claude -p\` in a temporary repository with the Stop hook"
  echo "pointed at \`weed hook claude\` through \`--settings\`, on model \`$model\`."
  echo
  echo "**Refused.** The working tree carried a half-finished merge:"
  echo
  echo '```'
  printf '%s\n' "$refused_table"
  echo '```'
  echo
  echo "The session tried to end and was sent back. The proof stopped it as soon"
  echo "as the block reached the transcript; left alone the hook refuses every"
  echo "stop over the same tree, so the turn cannot end at all. This is what the"
  echo "session received, verbatim:"
  echo
  echo '```'
  printf '%s\n' "$feedback"
  echo '```'
  echo
  echo "**Allowed.** The merge finished, and weed had nothing to say:"
  echo
  echo '```'
  printf '%s\n' "$allowed_table"
  echo '```'
  echo
  echo "The session ended on its own, exit 0, with no Stop hook feedback at all."
} > "$capture"

awk -v heading="$heading" -v body="$capture" '
  $0 == heading {
    print; print ""
    while ((getline line < body) > 0) print line
    skipping = 1
    next
  }
  skipping && /^## / { print ""; skipping = 0 }
  !skipping { print }
' "$doc" > "$doc.written"
mv "$doc.written" "$doc"

command -v trash >/dev/null 2>&1 && trash "$work" || echo "the transcripts are at $work"

echo "scripts/proof/claude-stop.sh: a real session was refused over a tree that blocks and ended on its own once it was clean; $doc updated"
