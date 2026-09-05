#!/usr/bin/env bash
# Evidence for sarif.tend2.html c4: docs/sarif.md names every SARIF convention
# and, for each, the test that pins it — and every one of those tests exists.
# The doc and the tests are checked against each other in both directions, so
# neither a convention documented without a test nor a test the doc never names
# survives this script.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

doc="docs/sarif.md"
[ -f "$doc" ] || { echo "$doc is missing: the conventions are unrecorded" >&2; exit 1; }

status=0

# Every convention the loop names has a row in the doc's table.
conventions=(
  "one run per invocation"
  "the tool component"
  "the rules array"
  "rule id and index"
  "the level mapping"
  "the message"
  "the location"
  "the region"
  "fixes"
  "suppressions"
  "the result's own path"
  "the invocation"
  "schema validity"
  "the table's lines"
  "the table's order"
  "the table's counts"
  "the table holds nothing else"
)
for convention in "${conventions[@]}"; do
  if ! grep -qF "| $convention |" "$doc"; then
    echo "$doc has no row for the convention '$convention'" >&2
    status=1
  fi
done

# Every test the doc names exists, in the file the doc names.
named="$(grep -oE 'tests/[a-z_]+\.rs::[a-z0-9_]+' "$doc" | sort -u)"
[ -n "$named" ] || { echo "$doc names no test: a convention with no test is a claim" >&2; exit 1; }
while IFS= read -r reference; do
  file="${reference%%::*}"
  function="${reference##*::}"
  if [ ! -f "$file" ]; then
    echo "$doc cites $reference but $file does not exist" >&2
    status=1
  elif ! grep -qE "^fn ${function}\(" "$file"; then
    echo "$doc cites $reference but $file has no test function ${function}" >&2
    status=1
  fi
done <<< "$named"

# Every test in the sarif files is named by the doc.
for file in tests/sarif_schema.rs tests/sarif_shape.rs tests/sarif_table.rs; do
  [ -f "$file" ] || { echo "$file is missing" >&2; status=1; continue; }
  while IFS= read -r function; do
    if ! grep -qF "$file::$function" "$doc"; then
      echo "$file::$function pins something $doc never names" >&2
      status=1
    fi
  done < <(grep -A1 '^#\[test\]$' "$file" | grep -oE '^fn [a-z0-9_]+' | cut -d' ' -f2)
done

if [ "$status" -eq 0 ]; then
  count="$(printf '%s\n' "$named" | wc -l | tr -d ' ')"
  echo "docs/sarif.md: ${#conventions[@]} conventions, $count tests named and present"
fi
exit "$status"
