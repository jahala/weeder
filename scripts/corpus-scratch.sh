#!/usr/bin/env bash
# Fetch every repository `docs/calibration/corpus.toml` pins into a scratch
# directory, and print what was fetched.
#
# The evidence scripts need git's own answer about the history the calibration
# judged, and since the corpus names no checkout on this machine there is no
# checkout to ask. So they ask the same thing the measurement asks: the pinned
# commit, fetched from the source the corpus names, with `refs/heads/calibration`
# on it and nothing else. A scratch whose head is not the pin stops the run.
#
# usage: bash scripts/corpus-scratch.sh <scratch-dir> [corpus-path]
# prints one line per repository: name<TAB>path<TAB>tip<TAB>source
set -euo pipefail

scratch="${1:?a scratch directory is required}"
root="$(cd "$(dirname "$0")/.." && pwd)"
corpus="${2:-$root/docs/calibration/corpus.toml}"

command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }
command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the corpus cannot be read" >&2
  exit 3
}
[ -f "$corpus" ] || { echo "$corpus is missing: there are no repositories to fetch" >&2; exit 3; }

mkdir -p "$scratch"
entries="$scratch/.corpus"
python3 - "$corpus" > "$entries" <<'PY'
import re
import sys

text = open(sys.argv[1], encoding="utf-8").read()
for block in text.split("[[repo]]")[1:]:
    fields = {}
    for key in ("name", "source", "tip"):
        found = re.search(rf'^\s*{key}\s*=\s*"([^"]+)"', block, re.M)
        if found:
            fields[key] = found.group(1)
    if len(fields) == 3:
        print("{name}\t{source}\t{tip}".format(**fields))
PY
[ -s "$entries" ] || { echo "$corpus names no repository" >&2; exit 3; }

while IFS=$'\t' read -r name source tip; do
  path="$scratch/$name"
  if [ ! -d "$path/.git" ]; then
    mkdir -p "$path"
    git -C "$path" init --quiet
    if ! git -C "$path" fetch --quiet --no-tags "$source" "$tip"; then
      echo "$name: $source would not hand over $tip. the corpus pins a commit, and a source that no longer has it cannot be judged." >&2
      exit 3
    fi
    git -C "$path" update-ref refs/heads/calibration "$tip"
  fi
  head="$(git -C "$path" rev-parse refs/heads/calibration)"
  if [ "$head" != "$tip" ]; then
    echo "$name: the scratch is at $head and the corpus pins $tip" >&2
    exit 3
  fi
  printf '%s\t%s\t%s\t%s\n' "$name" "$path" "$tip" "$source"
done < "$entries"
