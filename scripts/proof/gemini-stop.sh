#!/usr/bin/env bash
# Evidence for hooks.tend2.html c5: a real Gemini CLI session, with weed on the
# AfterAgent hook, refuses a premature done and says why in weed's own words —
# and ends on its own once the tree is clean.
#
# This is the owner's to run. Gemini CLI on this machine has no auth method set,
# so no worker can start a real session from it; the script says so and leaves
# with 3 rather than pretending. Everything else mirrors scripts/proof/claude-stop.sh.
#
# The hook is handed to gemini through GEMINI_CLI_SYSTEM_SETTINGS_PATH, so the
# owner's own ~/.gemini/settings.json is read for auth and never written to, and
# --skip-trust lets a hook run in the temporary repository the proof builds.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

doc="docs/proof-2026-09.md"
heading="## gemini cli — the after-agent hook, run for real"
limit="${WEED_PROOF_LIMIT:-90}"
marker="weed hook refused"

for tool in gemini git jq; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "$tool is not on PATH: the proof needs a real session, not a stand-in" >&2
    exit 3
  }
done
[ -f "$doc" ] || { echo "$doc is missing: there is nowhere to record the run" >&2; exit 3; }

# Whether a session can start at all. An unauthenticated gemini leaves with its
# own code and says which variable or setting is missing, and that is a fact
# about this machine rather than a failure of the hook.
probe="$(mktemp)"
if ! gemini -p "Reply with exactly: ready." -o json --skip-trust > "$probe" 2>&1; then
  reason="$(jq -r '.error.message // empty' "$probe" 2>/dev/null || true)"
  [ -n "$reason" ] || reason="$(head -3 "$probe")"
  echo "gemini cannot start a session here: $reason" >&2
  echo "the owner signs in and runs this script; no worker can." >&2
  exit 3
fi

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
{"hooks": {"AfterAgent": [{"matcher": "", "hooks": [{"name": "weed", "type": "command", "command": "$weed hook gemini"}]}]}}
JSON

session() { # $1 transcript, $2 seconds before it is stopped, $3 whether to wait for the marker
  local transcript="$1" seconds="$2" until_blocked="$3" pid waited=0
  ( cd "$work" && GEMINI_CLI_SYSTEM_SETTINGS_PATH="$work/settings.json" gemini \
      -p "Reply with exactly: done. Change nothing." \
      -o stream-json --skip-trust --approval-mode plan ) > "$transcript" 2>&1 &
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

if ! grep -q "$marker" "$work/refused.jsonl"; then
  echo "the session ended over a tree weed refuses, and said nothing about it" >&2
  status=1
fi
if ! grep -q 'G1' "$work/refused.jsonl"; then
  echo "the session was blocked, but the reason it received names no finding" >&2
  status=1
fi
feedback="$(grep -o "[^\"]*${marker}[^\"]*" "$work/refused.jsonl" | head -1 || true)"

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

capture="$work/capture.md"
{
  echo "### $(date +%Y-%m-%d) — gemini-cli $(gemini --version | head -1), $("$weed" --version)"
  echo
  echo "Both halves ran \`gemini -p\` in a temporary repository, with the AfterAgent"
  echo "hook handed over through \`GEMINI_CLI_SYSTEM_SETTINGS_PATH\`."
  echo
  echo "**Refused.** The working tree carried a half-finished merge:"
  echo
  echo '```'
  printf '%s\n' "$refused_table"
  echo '```'
  echo
  echo "The session tried to end and was sent back with weed's reason as its next"
  echo "request, which is what gemini does with a blocking AfterAgent decision:"
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
  echo "The session ended on its own, exit 0, with no block at all."
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

echo "scripts/proof/gemini-stop.sh: a real session was refused over a tree that blocks and ended on its own once it was clean; $doc updated"
