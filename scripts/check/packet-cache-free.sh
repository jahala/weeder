#!/usr/bin/env bash
# Evidence for calibration-audit: a case packet is derived from the pinned corpus
# and the miss list alone, never from whatever a campaign left in a cache. The
# generator runs twice for the audit's seed, once with an empty cache directory
# and once with the cache this machine already holds, and the two sets of case
# files have to be the same names and the same bytes.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
audit="docs/calibration-audit-blind-2026-09.md"
[ -f "$audit" ] || { echo "$audit is missing" >&2; exit 1; }
seed="$(sed -n 's/^Seed:[[:space:]]*//p' "$audit" | head -1)"
[ -n "$seed" ] || { echo "$audit names no seed" >&2; exit 1; }
scratch="$(mktemp -d)"
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT
mkdir -p "$scratch/empty-cache" "$scratch/a" "$scratch/b"
WEED_RECALL_CACHE="$scratch/empty-cache" cargo xtask audit-packet --seed "$seed" --dir "$scratch/a" >/dev/null
cargo xtask audit-packet --seed "$seed" --dir "$scratch/b" >/dev/null
count_a="$(ls "$scratch/a"/*.md | wc -l | tr -d ' ')"
count_b="$(ls "$scratch/b"/*.md | wc -l | tr -d ' ')"
[ "$count_a" = "$count_b" ] || { echo "an empty cache gave $count_a case packets and the machine's cache gave $count_b" >&2; exit 1; }
[ "$count_a" = "40" ] || { echo "$count_a case packets, not 40" >&2; exit 1; }
if ! diff -qr "$scratch/a" "$scratch/b" >/dev/null; then
  echo "the case packets differ between an empty cache and the machine's cache:" >&2
  diff -qr "$scratch/a" "$scratch/b" >&2 | head -10
  exit 1
fi
kept="fixtures/adversarial/calibration-audit/blind-2026-09/cases"
if ! diff -qr "$scratch/a" "$kept" >/dev/null; then
  echo "the packets in the tree differ from what an empty cache derives for seed $seed:" >&2
  diff -qr "$scratch/a" "$kept" >&2 | head -10
  exit 1
fi
echo "40 case packets derive the same bytes from an empty cache, the machine's cache and the tree, for seed $seed"
