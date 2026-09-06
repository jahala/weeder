#!/usr/bin/env bash
# Evidence for scan.tend2.html c6: docs/tend2-seam.md is a proposal that can be
# checked rather than believed.
#
# Three things are asked of the document. It shows a tend2 check whose evidence
# is built on `weed scan --format sarif`. It says how tend2's verifier could call
# `weed check` where it now calls its own `mock.ts`. And every command in its
# table is run here, in the place the table names, and compared against the exit
# code the table claims — so a line in the doc cannot outlive the binary.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

doc="docs/tend2-seam.md"
[ -f "$doc" ] || { echo "$doc is missing: the seam is unproposed" >&2; exit 1; }

status=0

# The scan half: a tend2 check line, and the command its evidence is built on.
if ! grep -qE '^- \[[ x!~]\] \(code\) .+ · .+$' "$doc"; then
  echo "$doc shows no tend2 check line, so it proposes no evidence" >&2
  status=1
fi
for phrase in "weed scan --format sarif" "mock.ts" "weed check --strict --format sarif"; do
  if ! grep -qF "$phrase" "$doc"; then
    echo "$doc never names \`$phrase\`" >&2
    status=1
  fi
done

cargo build --quiet --bin weed
binary="$root/target/debug/weed"

# A repository with nothing to judge, and one whose change deletes a test. Both
# are real git repositories built here, because the exit code the doc claims has
# to come from the binary rather than from this script's opinion of it.
scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT

git_quiet() {
  git -c user.name="weed evidence" -c user.email="evidence@weed.invalid" \
      -c commit.gpgsign=false -C "$1" "${@:2}" >/dev/null 2>&1
}

clean="$scratch/clean"
mkdir -p "$clean"
git_quiet "$clean" init --initial-branch=main
mkdir -p "$clean/src" "$clean/tests"
cat > "$clean/src/parser.ts" <<'TS'
export function parse(line: string): string[] {
  return line.split(",");
}
TS
cat > "$clean/tests/parser.test.ts" <<'TS'
import { parse } from "../src/parser";

test("a line splits on the comma", () => {
  expect(parse("a,b")).toEqual(["a", "b"]);
});
TS
git_quiet "$clean" add -A
git_quiet "$clean" commit -m "the parser and its test"

deleted="$scratch/deleted"
cp -R "$clean" "$deleted"
rm "$deleted/tests/parser.test.ts"
git_quiet "$deleted" add -A

trim() {
  printf '%s' "$1" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'
}

where() {
  case "$1" in
    "this repository") echo "$root" ;;
    "a repository with nothing to judge") echo "$clean" ;;
    "a repository whose change deletes a test") echo "$deleted" ;;
    *) echo "" ;;
  esac
}

# Every row of the doc's own table: the command, the place, the exit code.
rows=0
while IFS='|' read -r _ command place expected _; do
  command="$(trim "$(printf '%s' "$command" | tr -d '`')")"
  place="$(trim "$place")"
  expected="$(trim "$expected")"
  case "$command" in weed\ *) ;; *) continue ;; esac
  case "$expected" in ''|*[!0-9]*) continue ;; esac

  directory="$(where "$place")"
  if [ -z "$directory" ]; then
    echo "$doc runs \`$command\` in '$place', and this script knows no such place" >&2
    status=1
    continue
  fi

  rows=$((rows + 1))
  # The doc names the binary the way an operator types it; the first word is
  # swapped for the build under test and the rest is passed through untouched.
  set +e
  (cd "$directory" && "$binary" ${command#weed }) >/dev/null 2>&1
  code=$?
  set -e
  if [ "$code" -ne "$expected" ]; then
    echo "$doc says \`$command\` leaves with $expected in $place, and it left with $code" >&2
    status=1
  fi
done < "$doc"

if [ "$rows" -eq 0 ]; then
  echo "$doc shows no command table, so nothing in it was checked" >&2
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo "docs/tend2-seam.md: the scan seam and the mock.ts proposal are both shown, and $rows commands left with the codes it claims"
fi
exit "$status"
