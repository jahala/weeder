#!/usr/bin/env bash
# Can the new evidence fail? Mutate the report three ways and check each script
# refuses it, then put the report back exactly as it was.
set -uo pipefail
cd "$(dirname "$0")/.."
report="docs/calibration-2026-09.md"
backup="$(mktemp)"
cp "$report" "$backup"
restore() { cp "$backup" "$report"; }
trap restore EXIT

fail=0
try() {
  local what="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    echo "NOT CAUGHT: $what"
    fail=1
  else
    echo "caught: $what"
  fi
  restore
}

python3 - "$report" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.replace("69 moved from C1 at block level to C3 at warn level", "12 moved from C1 at block level to C3 at warn level")
open(p, "w").write(s)
PY
try "a C1-to-C3 count the table does not support" bash scripts/check/c3-split.sh

python3 - "$report" <<'PY'
import sys
p = sys.argv[1]
lines = open(p).read().splitlines(keepends=True)
out = []
dropped = False
for line in lines:
    if not dropped and line.startswith("| tilth | `") and "| workflow | C1 | C3 | warn |" in line:
        dropped = True
        continue
    out.append(line)
open(p, "w").writelines(out)
PY
try "a row taken out of the split table" bash scripts/check/c3-split.sh

python3 - "$report" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.replace(
    "weed ships as a gate, pending the independent re-grade:",
    "weed ships as a gate:",
    1,
)
open(p, "w").write(s)
PY
try "a verdict promoted past the re-grade" env WEED_CALIBRATION_METRIC=/dev/null bash scripts/check/calibration-bar.sh

python3 - "$report" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.replace("The window ends at `f5c0afa97c6666a3d68dcbd965a4db5a44bc0905`", "The window ends at `deadbeef`")
open(p, "w").write(s)
PY
try "a report naming a commit the corpus does not pin" bash scripts/check/corpus-pinned.sh

exit "$fail"
