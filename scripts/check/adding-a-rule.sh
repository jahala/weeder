#!/usr/bin/env bash
# Evidence for authoring.tend2.html c3: docs/adding-a-rule.md walks one rule
# through every file a new rule touches, CONTRIBUTING.md points at it, and
# weeder's own scan finds nothing stale in it.
#
# Three checks. The built binary's `scan` reports no R1 result on the page, so
# every path, symbol, command and flag R1 can answer for is one the tree still
# has. Every path the page cites in backticks exists, and every test it cites
# as `file::function` is a function in that file, which holds the citations R1
# has no authority over. And CONTRIBUTING.md names the page.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

doc="docs/adding-a-rule.md"
[ -f "$doc" ] || { echo "$doc is missing: the walk through a new rule is unwritten" >&2; exit 1; }

command -v cargo >/dev/null 2>&1 || { echo "cargo is not on PATH, so weeder cannot be built" >&2; exit 3; }
command -v jq >/dev/null 2>&1 || { echo "jq is not on PATH, so the SARIF log cannot be read" >&2; exit 3; }

status=0

# The walk covers each place a rule is written, from the catalogue row to the
# recall campaign's reader.
for place in \
  src/core/catalogue.rs \
  src/core/rules/check/g1.rs \
  src/core/rules/check/mod.rs \
  'fixtures/adversarial/<RULE>/<lang>/<case>/' \
  tests/catalogue_matrix.rs \
  tests/rule_g1.rs \
  docs/rules.md \
  tests/config_levels.rs \
  xtask/src/mutate/inject.rs \
  xtask/src/mutate/shape.rs; do
  if ! grep -qF "\`$place\`" "$doc"; then
    echo "$doc never cites $place, so the walk skips a place a rule is written" >&2
    status=1
  fi
done

if ! grep -qF "docs/adding-a-rule.md" CONTRIBUTING.md; then
  echo "CONTRIBUTING.md does not point at $doc" >&2
  status=1
fi

# Every path in backticks exists, and every `file::function` is a function in
# that file. A token with a slash or a known extension is a path; a token
# carrying a placeholder or a glob is a pattern and is left alone.
cited="$(grep -oE '`[A-Za-z0-9_./:-]+`' "$doc" | tr -d '`' | sort -u)"
paths=0
while IFS= read -r token; do
  path="${token%%::*}"
  case "$path" in
    */* | *.rs | *.md | *.toml | *.sh | *.json) ;;
    *) continue ;;
  esac
  paths=$((paths + 1))
  if [ ! -e "$path" ]; then
    echo "$doc cites $path, and the tree has no such path" >&2
    status=1
    continue
  fi
  if [ "$token" != "$path" ]; then
    function="${token##*::}"
    if ! grep -qE "fn ${function}\(" "$path"; then
      echo "$doc cites $token, and $path has no function $function" >&2
      status=1
    fi
  fi
done <<< "$cited"
[ "$paths" -gt 0 ] || { echo "$doc cites no path at all" >&2; exit 1; }

# The page cites the fixtures as a pattern, because R1 cannot see a specimen
# path. The pattern is held to the rule it walks: G1 keeps every cell of it.
cells=0
for lang in ts py rs go; do
  for case in fire silent; do
    for side in before after; do
      cell="fixtures/adversarial/G1/$lang/$case/$side"
      [ -d "$cell" ] || { echo "$doc walks G1, and $cell is missing" >&2; status=1; }
      cells=$((cells + 1))
    done
  done
done

# Tests cited by bare name are functions somewhere under tests/ or xtask/.
for function in $(grep -oE '`(every|a_rule)_[a-z0-9_]+`' "$doc" | tr -d '`' | sort -u); do
  if ! grep -rqE "fn ${function}\(" tests xtask/src; then
    echo "$doc cites the test $function, and no test of that name exists" >&2
    status=1
  fi
done

# weeder's own scan finds nothing stale on the page.
cargo build --quiet --bin weeder
stale="$(./target/debug/weeder scan --format sarif | jq -r --arg doc "$doc" \
  '.runs[0].results[]
   | select(.ruleId == "R1")
   | select(.locations[0].physicalLocation.artifactLocation.uri == $doc)
   | "\(.locations[0].physicalLocation.region.startLine): \(.message.text)"')"
if [ -n "$stale" ]; then
  echo "weeder scan reports R1 on $doc:" >&2
  echo "$stale" >&2
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo "$doc: $paths paths cited and present, G1's $cells fixture sides present, no R1 result, CONTRIBUTING.md points at it"
fi
exit "$status"
